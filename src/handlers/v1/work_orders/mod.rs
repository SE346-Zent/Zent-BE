pub mod create;
pub mod list;
pub mod get_details;
pub mod assign;
pub mod complete;
pub mod refuse;
pub mod start;
pub mod approve_refusal;
pub mod deny_refusal;
pub mod history;
pub mod cancel;

pub use create::create;
pub use list::list;
pub use get_details::get_details;
pub use assign::assign;
pub use complete::complete;
pub use refuse::refuse;
pub use start::start;
pub use approve_refusal::approve_refusal;
pub use deny_refusal::deny_refusal;
pub use history::history;
pub use cancel::cancel;

// Re-export __path_* items for utoipa OpenApi derive
pub use create::__path_create;
pub use list::__path_list;
pub use get_details::__path_get_details;
pub use assign::__path_assign;
pub use complete::__path_complete;
pub use refuse::__path_refuse;
pub use start::__path_start;
pub use approve_refusal::__path_approve_refusal;
pub use deny_refusal::__path_deny_refusal;
pub use history::__path_history;

use axum::{Router, middleware};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;
use redis::AsyncCommands;
use sea_orm::{DatabaseConnection, prelude::Uuid, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, ColumnTrait, TransactionTrait, ActiveModelTrait, Set};
use tracing::{info, error};
use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::entities::roles::Role;
use crate::entities::work_orders as work_orders_ent;
use crate::entities::{products, work_order_symptoms, users, work_order_state_history};
use crate::extractor::role_check::require_role;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::list_response::WorkOrderListItem;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::requests::pagination::PaginationRequest;
use crate::services::v1::work_orders::list as list_svc;
use crate::infrastructure::cache::ValkeyClient;

/// Sentinel value stored during the idempotency claim window.
pub(crate) const IDEMPOTENCY_PENDING: &str = "__PENDING__";

pub fn work_orders_router(state: AppState) -> Router<AppState> {
    let customer_routes = Router::new()
        .route("/", axum::routing::post(create))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    let tech_routes = Router::new()
        .route("/{id}/start", axum::routing::post(start))
        .route("/{id}/refuse", axum::routing::post(refuse))
        .route("/{id}/complete", axum::routing::post(complete))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    let admin_routes = Router::new()
        .route("/{id}/assign", axum::routing::post(assign))
        .route("/{id}/cancel", axum::routing::post(cancel))
        .route("/{id}/refusal/approve", axum::routing::post(approve_refusal))
        .route("/{id}/refusal/deny", axum::routing::post(deny_refusal))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    let list_route = Router::new()
        .route("/", axum::routing::get(list));

    Router::new()
        .route("/{id}", axum::routing::get(get_details))
        .route("/{id}/history", axum::routing::get(history))
        .merge(list_route)
        .merge(customer_routes)
        .merge(tech_routes)
        .merge(admin_routes)
}

//pub async fn cancel() -> axum::http::StatusCode { axum::http::StatusCode::NOT_IMPLEMENTED }

pub(crate) async fn fetch_paginated_work_orders(
    db: Arc<DatabaseConnection>,
    valkey_client: Option<Arc<ValkeyClient>>,
    lookup_tables: Arc<LookupTables>,
    pagination: PaginationRequest,
    cache_key_prefix: &str,
    technician_id: Option<Uuid>,
    province_filter: Option<String>,
    customer_id: Option<Uuid>,
) -> Result<axum::Json<ApiResponse<Vec<WorkOrderListItem>>>, AppError> {
    let mut conn_opt = None;
    let mut full_cache_key = String::new();

    if let Some(client) = valkey_client.as_ref() {
        let mut conn = client.get_connection();
        let gen: u64 = conn.get("cache:work_orders:generation").await.unwrap_or(0);
        full_cache_key = format!(
            "cache:work_orders:gen:{}:prefix:{}:p:{}:l:{}",
            gen, cache_key_prefix, pagination.page, pagination.limit
        );
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&full_cache_key).await {
            if let Ok((data, meta)) = serde_json::from_str::<(Vec<WorkOrderListItem>, PaginationResponse)>(&cached_json) {
                return Ok(axum::Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)));
            }
        }
        conn_opt = Some(conn);
    }

    let mut query = work_orders_ent::Entity::find();
    if let Some(tech_id) = technician_id { query = query.filter(work_orders_ent::Column::TechnicianId.eq(tech_id)); }
    if let Some(province) = province_filter { query = query.filter(work_orders_ent::Column::Province.eq(province)); }
    if let Some(cust_id) = customer_id { query = query.filter(work_orders_ent::Column::CustomerId.eq(cust_id)); }

    let paginator = query.clone().order_by_desc(work_orders_ent::Column::CreatedAt).paginate(db.as_ref(), pagination.limit);
    let total_records = paginator.num_items().await?;

    let models_with_related = query
        .order_by_desc(work_orders_ent::Column::CreatedAt)
        .find_also_related(products::Entity).find_also_related(work_order_symptoms::Entity)
        .offset((pagination.page - 1) * pagination.limit).limit(pagination.limit)
        .all(db.as_ref()).await?;

    let (data, meta) = list_svc::decide_list(models_with_related, &lookup_tables, &pagination, total_records);

    if let Some(mut conn) = conn_opt {
        if let Ok(cached_val) = serde_json::to_string(&(&data, &meta)) {
            let _: () = conn.set_ex(&full_cache_key, cached_val, 300).await.unwrap_or_default();
        }
    }

    Ok(axum::Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)))
}

/// Cron-triggered cleanup: cancels all pending unassigned work orders that have exceeded
/// the threshold defined in the policy and notifies the customer.
pub async fn run_cleanup(
    db: &DatabaseConnection,
    luts: &LookupTables,
    rabbitmq_opt: &Option<std::sync::Arc<lapin::Connection>>,
) -> Result<(), anyhow::Error> {
    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;

    // Get threshold from policy cache (LUT)
    let threshold_hours: i64 = luts.policies.get("unassigned_cleanup_threshold_hours").and_then(|v| v.parse().ok()).unwrap_or(3);

    let pending_status_id = *luts.work_order_statuses_by_name.get("Pending").unwrap_or_else(|| luts.work_order_statuses_by_name.get("Pending assignment").unwrap());
    let closed_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap();

    let now = Utc::now();
    let cutoff_time = now - chrono::Duration::hours(threshold_hours);

    // Find WOs that are still pending assignment and were created before the cutoff time
    let target_wos = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::WorkOrderStatusId.eq(pending_status_id))
        .filter(work_orders_ent::Column::TechnicianId.is_null())
        .filter(work_orders_ent::Column::CreatedAt.lte(cutoff_time))
        .all(db).await?;

    if target_wos.is_empty() { return Ok(()); }

    for wo in target_wos {
        let mut wo_active: work_orders_ent::ActiveModel = wo.clone().into();
        wo_active.work_order_status_id = Set(closed_status_id);
        wo_active.updated_at = Set(now);

        let history = work_order_state_history::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            work_order_id: Set(wo.id),
            from_status_id: Set(Some(pending_status_id)),
            to_status_id: Set(closed_status_id),
            changed_by_id: Set(system_user_id),
            changed_at: Set(now),
        };

        match db.transaction::<_, (), anyhow::Error>(|txn| Box::pin(async move {
            wo_active.update(txn).await.map_err(|e| anyhow::anyhow!(e))?;
            history.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?;
            Ok(())
        })).await {
            Ok(_) => {
                info!("Cancelled unassigned WO {} successfully", wo.work_order_number);
                if let Some(rmq) = rabbitmq_opt.as_ref() {
                    let cust = users::Entity::find_by_id(wo.customer_id).one(db).await.unwrap_or_default();
                    if let Some(c) = cust {
                        let _ = crate::services::v1::core::email_service::send_email(
                            rmq,
                            &c.email,
                            "Work Order Cancelled",
                            &format!("Dear {},\n\nYour work order {} has been cancelled because we could not assign a technician within the required time. We apologize for the inconvenience.", c.full_name, wo.work_order_number),
                        ).await;
                    }
                }
            }
            Err(e) => error!("Cleanup failed for WO {}: {}", wo.work_order_number, e),
        }
    }
    Ok(())
}

pub(crate) async fn try_auto_assign_single(
    db: Arc<DatabaseConnection>,
    luts: Arc<LookupTables>,
    wo: work_orders_ent::Model,
    valkey_client: Option<Arc<ValkeyClient>>,
    rabbitmq_opt: Option<Arc<lapin::Connection>>,
    templates: Option<Arc<std::collections::HashMap<String, String>>>,
) -> bool {
    let tech_role_id = match luts.roles_by_name.get("Technician") {
        Some(id) => *id,
        None => { tracing::warn!("Technician role not found"); return false; }
    };
    let policies = &luts.policies;
    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned").unwrap_or_else(|| luts.work_order_statuses_by_name.get("Pending").unwrap());
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap_or_else(|| luts.work_order_statuses_by_name.get("Pending").unwrap());
    let cfg = crate::core::config::AppConfig::get();
    let system_user_id = cfg.system_user_id;
    let province = wo.province.clone();

    let technicians = match users::Entity::find().filter(users::Column::RoleId.eq(tech_role_id)).filter(users::Column::Province.eq(&province)).all(db.as_ref()).await {
        Ok(t) => t, Err(e) => { tracing::warn!("Failed to find technicians: {}", e); return false; }
    };
    if technicians.is_empty() {
        tracing::info!("Auto-assign: no technicians in province '{}' for WO {} — admin needed", province, wo.work_order_number);
        return false;
    }
    let tech_ids: Vec<Uuid> = technicians.iter().map(|t| t.id).collect();
    let agendas = match work_orders_ent::Entity::find().filter(work_orders_ent::Column::TechnicianId.is_in(tech_ids)).filter(work_orders_ent::Column::WorkOrderStatusId.ne(done_status_id)).all(db.as_ref()).await {
        Ok(a) => a, Err(e) => { tracing::warn!("Failed to load agendas: {}", e); return false; }
    };
    let mut technician_agendas: HashMap<Uuid, Vec<work_orders_ent::Model>> = HashMap::new();
    for a in agendas { if let Some(tid) = a.technician_id { technician_agendas.entry(tid).or_default().push(a); } }

    let effect = match crate::services::v1::work_orders::auto_assign::decide_auto_assign(wo.clone(), technicians, technician_agendas, &policies, assigned_status_id, done_status_id, system_user_id) {
        Ok(Some(eff)) => eff,
        Ok(None) => { tracing::info!("Auto-assign: no suitable tech for WO {} — admin needed", wo.work_order_number); return false; }
        Err(e) => { tracing::error!("Auto-assign failed for WO {}: {}", wo.work_order_number, e); return false; }
    };
    let assigned_tech_id = effect.work_order.technician_id.clone().unwrap().unwrap();

    if let Err(e) = db.transaction::<_, (), anyhow::Error>(|txn| Box::pin(async move {
        effect.work_order.update(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        effect.state_history.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    })).await { tracing::error!("Auto-assign tx failed for WO {}: {}", wo.work_order_number, e); return false; }

    tracing::info!("Auto-assigned WO {} to {}", wo.work_order_number, assigned_tech_id);
    if let (Some(rmq), Some(tmpl)) = (rabbitmq_opt.as_ref(), templates.as_ref()) {
        let cust = users::Entity::find_by_id(wo.customer_id).one(db.as_ref()).await.unwrap_or_default();
        let tech = users::Entity::find_by_id(assigned_tech_id).one(db.as_ref()).await.unwrap_or_default();
        if let (Some(c), Some(t)) = (cust, tech) {
            let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(rmq, tmpl, &c.email, &c.full_name, &wo.work_order_number, &t.full_name, &wo.appointment.to_string()).await;
        }
    }
    if let Some(client) = valkey_client.as_ref() {
        let mut conn = client.get_connection();
        let _: u64 = conn.incr::<&str, i32, u64>("cache:work_orders:generation", 1).await.unwrap_or_default();
    }
    true
}
