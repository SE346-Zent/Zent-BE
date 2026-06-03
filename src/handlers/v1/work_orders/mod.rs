//! Work Order API Handlers (v1)
//!
//! This module provides the entry points for work order related API requests,
//! including routing, caching strategies, and integration with domain services.
pub mod create;
pub mod list;
pub mod get_details;
pub mod assign;
pub mod complete;
pub mod reassign;
pub mod refuse;
pub mod start;
pub mod approve_refusal;
pub mod deny_refusal;
pub mod history;
pub mod cancel;
pub mod rate;
pub mod change_appointment;
pub mod reject_form_list;
pub mod reject_form_detail;
pub mod check_geofence;
pub mod edit;
pub mod get_metrics;

pub use change_appointment::change_appointment;
pub use rate::rate;
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
pub use reassign::reassign;
pub use cancel::cancel;
pub use reject_form_list::reject_form_list;
pub use reject_form_detail::reject_form_detail;
pub use check_geofence::check_geofence;
pub use edit::edit;
pub use get_metrics::get_technician_metrics;

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
pub use reassign::__path_reassign;
pub use cancel::__path_cancel;
pub use rate::__path_rate;
pub use change_appointment::__path_change_appointment;
pub use reject_form_list::__path_reject_form_list;
pub use reject_form_detail::__path_reject_form_detail;
pub use check_geofence::__path_check_geofence;
pub use edit::__path_edit;
pub use get_metrics::__path_get_technician_metrics;


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
use crate::entities::{products, work_order_symptoms, users, work_order_state_history, work_order_ratings, warranties};
use crate::extractor::role_check::require_role;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::list_response::WorkOrderListItem;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::requests::pagination::PaginationRequest;
use crate::services::v1::work_orders::list as list_svc;
use crate::services::v1::work_orders::technician_stats::{TechnicianStatsInput, TechnicianStatsSnapshot};
use crate::infrastructure::cache::ValkeyClient;
use crate::services::v1::inventory::ports::ZeusInventoryClient;

/// Sentinel value stored during the idempotency claim window.
pub(crate) const IDEMPOTENCY_PENDING: &str = "__PENDING__";

async fn load_zeus_product_model(product_id: Uuid) -> Option<products::Model> {
    let base_url = match std::env::var("ZEUS_BASE_URL") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return None,
    };
    let api_key = match std::env::var("ZEUS_API_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return None,
    };

    let client = crate::infrastructure::clients::zeus::ZeusClient::new(base_url, api_key);
    let zeus_prod = match client.get_product(product_id).await {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(products::Model {
        id: zeus_prod.id,
        product_model_code: zeus_prod.product_model_code,
        customer_id: zeus_prod.customer_id,
        product_name: zeus_prod.product_name,
        serial_number: zeus_prod.serial_number,
        created_at: zeus_prod.created_at,
        updated_at: zeus_prod.updated_at,
        deleted_at: None,
    })
}

/// Initialize and configure the work order sub-router with role-based access control.

pub fn work_orders_router(state: AppState) -> Router<AppState> {
    let customer_routes = Router::new()
        .route("/", axum::routing::post(create))
        .route("/{id}/rate", axum::routing::post(rate))
        .route("/{workOrderNumber}/edit", axum::routing::post(edit))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    let tech_routes = Router::new()
        .route("/{id}/start", axum::routing::post(start))
        .route("/{id}/refuse", axum::routing::post(refuse))
        .route("/{id}/complete", axum::routing::post(complete))
        .route("/{id}/geofence", axum::routing::post(check_geofence))
        .route("/technician/metrics", axum::routing::get(get_technician_metrics))

        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    let admin_routes = Router::new()
        .route("/{id}/assign", axum::routing::post(assign))
        .route("/{id}/refusal/approve", axum::routing::post(approve_refusal))
        .route("/{id}/refusal/deny", axum::routing::post(deny_refusal))
        .route("/{id}/reassign", axum::routing::post(reassign))
        .route("/reject_forms", axum::routing::get(reject_form_list))
        .route("/reject_forms/{id}", axum::routing::get(reject_form_detail))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    let list_route = Router::new()
        .route("/", axum::routing::get(list));

    Router::new()
        .route("/{id}", axum::routing::get(get_details))
        .route("/{id}/history", axum::routing::get(history))
        .route("/cancel", axum::routing::post(cancel))
        .merge(list_route)
        .merge(customer_routes)
        .merge(tech_routes)
        .merge(admin_routes)
}

//pub async fn cancel() -> axum::http::StatusCode { axum::http::StatusCode::NOT_IMPLEMENTED }

/// Retrieve a work order model using a cache-first strategy with database fallback.

/// Load a work order model — cache-first, DB-fallback.
/// Checks `cache:work_order_model:{id}`, returns the raw `work_orders::Model` if found.
/// On cache miss, queries the database, stores the model in cache, and returns it.
/// Returns `AppError::NotFound` if the work order doesn't exist.
pub(crate) async fn get_cached_work_order_model(
    db: &DatabaseConnection,
    valkey_client: &Option<Arc<ValkeyClient>>,
    id: Uuid,
) -> Result<work_orders_ent::Model, AppError> {
    let model_cache_key = format!("cache:work_order_model:{}", id);

    // Cache hit — serve from RAM (but skip soft-deleted)
    if let Some(client) = valkey_client.as_ref() {
        if let Ok(mut conn) = client.get_connection().await {
            if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&model_cache_key).await {
                if let Ok(model) = serde_json::from_str::<work_orders_ent::Model>(&cached_json) {
                    if model.deleted_at.is_none() {
                        return Ok(model);
                    }
                }
            }
        }
    }

    // Cache miss — load from DB
    let model = work_orders_ent::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if model.deleted_at.is_some() {
        return Err(AppError::NotFound("Work order not found".to_string()));
    }

    // Populate cache for next time (TTL matches details cache)
    if let Some(client) = valkey_client.as_ref() {
        if let Ok(cached_val) = serde_json::to_string(&model) {
            if let Ok(mut conn) = client.get_connection().await {
                let _: () = conn
                    .set_ex(&model_cache_key, cached_val, 600)
                    .await
                    .unwrap_or_default();
            } else {
                tracing::warn!("Valkey unavailable — model cache write skipped");
            }
        }
    }

    Ok(model)
}

/// Load a work order by its business-friendly `work_order_number` (e.g. `WO-AB12CD`).
///
/// `work_order_number` has a unique index in MySQL, so the DB lookup is O(log n) and
/// we intentionally skip the cache here: customers typically reference a work order by
/// its number a small number of times, and keeping the cache key aligned with the
/// primary identifier (`id`) avoids a second cache-invalidation path.
pub(crate) async fn get_work_order_by_number(
    db: &DatabaseConnection,
    work_order_number: &str,
) -> Result<work_orders_ent::Model, AppError> {
    let trimmed = work_order_number.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("Work order number is required".to_string()));
    }
    let model = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::WorkOrderNumber.eq(trimmed))
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;
    Ok(model)
}

/// Update the write-through cache for a work order and invalidate related listing caches.

/// Write-through cache: after a successful mutation, re-fetch the work order with
/// its related entities, build the full WorkOrderDetails and cache the raw model,
/// then bump the generation counter so listing caches are invalidated.
///
/// This ensures `get_details` always serves from RAM and list queries never show stale data.
pub(crate) async fn write_through_work_order_cache(
    db: &DatabaseConnection,
    valkey_client: Option<Arc<ValkeyClient>>,
    lookup_tables: &LookupTables,
    wo_id: Uuid,
) {
    if valkey_client.is_none() {
        return;
    }
    // Re-fetch the work order with symptom relation so we can build WorkOrderDetails.
    if let Ok(Some((wo, symptom))) = work_orders_ent::Entity::find_by_id(wo_id)
        .find_also_related(work_order_symptoms::Entity)
        .one(db)
        .await
    {
        let product = load_zeus_product_model(wo.product_id).await;
        let client = valkey_client.as_ref().unwrap();
        let Ok(mut conn) = client.get_connection().await else { return; };

        // 1. Cache the raw model (for mutation handlers that need it)
        if let Ok(model_json) = serde_json::to_string(&wo) {
            let _: () = conn
                .set_ex::<_, _, ()>(
                    format!("cache:work_order_model:{}", wo_id),
                    model_json,
                    600,
                )
                .await
                .unwrap_or_default();
        }

        // 2. Cache the full WorkOrderDetails (for get_details read path)
        // Fetch warranty status for the product
        let warranty_status = warranties::Entity::find()
            .filter(warranties::Column::ProductId.eq(wo.product_id))
            .one(db)
            .await
            .ok()
            .flatten()
            .map(|w| {
                let now = chrono::Utc::now();
                if now > w.end_date {
                    "expired".to_string()
                } else {
                    w.warranty_status.clone()
                }
            })
            .unwrap_or_else(|| "none".to_string());

        // Fetch technician info for avatar
        let (technician_name, technician_avatar_name) = if let Some(tech_id) = wo.technician_id {
            users::Entity::find_by_id(tech_id)
                .one(db)
                .await
                .ok()
                .flatten()
                .map(|t| (Some(t.full_name.clone()), t.avatar_url))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

        // Fetch customer avatar
        let customer_avatar_name = users::Entity::find_by_id(wo.customer_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .and_then(|c| c.avatar_url);

        let details = crate::services::v1::work_orders::get_details::decide_get_details(
            wo, product, symptom, lookup_tables, Some(warranty_status), technician_name, technician_avatar_name, customer_avatar_name,
        );
        if let Ok(details_json) = serde_json::to_string(&details) {
            let _: () = conn
                .set_ex::<_, _, ()>(
                    format!("cache:work_order:{}", wo_id),
                    details_json,
                    600,
                )
                .await
                .unwrap_or_default();
        }

        // 3. Bump generation to invalidate listing caches
        let _: u64 = conn
            .incr::<&str, i32, u64>("cache:work_orders:generation", 1)
            .await
            .unwrap_or_default();
    }
}

/// Perform a paginated search for work orders with support for various filters and caching.

pub(crate) async fn fetch_paginated_work_orders(
    db: Arc<DatabaseConnection>,
    valkey_client: Option<Arc<ValkeyClient>>,
    lookup_tables: Arc<LookupTables>,
    pagination: PaginationRequest,
    cache_key_prefix: &str,
    technician_id: Option<Uuid>,
    province_filter: Option<String>,
    customer_id: Option<Uuid>,
    date_filter: Option<chrono::NaiveDate>,
) -> Result<axum::Json<ApiResponse<Vec<WorkOrderListItem>>>, AppError> {
    let mut conn_opt = None;
    let mut full_cache_key = String::new();

    if let Some(client) = valkey_client.as_ref() {
        if let Ok(mut conn) = client.get_connection().await {
            let gen: u64 = conn.get("cache:work_orders:generation").await.unwrap_or(0);
            full_cache_key = format!(
                "cache:work_orders:gen:{}:prefix:{}:p:{}:l:{}:d:{:?}",
                gen, cache_key_prefix, pagination.page, pagination.limit, date_filter
            );
            if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&full_cache_key).await {
                if let Ok((data, meta)) = serde_json::from_str::<(Vec<WorkOrderListItem>, PaginationResponse)>(&cached_json) {
                    return Ok(axum::Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)));
                }
            }
            conn_opt = Some(conn);
        }
    }

    let mut query = work_orders_ent::Entity::find().filter(work_orders_ent::Column::DeletedAt.is_null());
    if let Some(tech_id) = technician_id { query = query.filter(work_orders_ent::Column::TechnicianId.eq(tech_id)); }
    if let Some(province) = province_filter { query = query.filter(work_orders_ent::Column::Province.eq(province)); }
    if let Some(cust_id) = customer_id { query = query.filter(work_orders_ent::Column::CustomerId.eq(cust_id)); }
    if let Some(ref date) = date_filter {
        let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
        query = query.filter(work_orders_ent::Column::Appointment.between(start, end));
    }

    let paginator = query.clone().order_by_desc(work_orders_ent::Column::CreatedAt).paginate(db.as_ref(), pagination.limit);
    let total_records = paginator.num_items().await?;

    let models_with_related = query
        .order_by_desc(work_orders_ent::Column::CreatedAt)
        .find_also_related(work_order_symptoms::Entity)
        .offset((pagination.page - 1) * pagination.limit).limit(pagination.limit)
        .all(db.as_ref()).await?;

    let mut enriched_models = Vec::with_capacity(models_with_related.len());

    // Batch-fetch users to avoid N+1
    let customer_ids: Vec<Uuid> = models_with_related.iter().map(|(wo, _)| wo.customer_id).collect();
    let technician_ids: Vec<Uuid> = models_with_related.iter().filter_map(|(wo, _)| wo.technician_id).collect();

    let mut all_user_ids = customer_ids.clone();
    all_user_ids.extend_from_slice(&technician_ids);
    all_user_ids.sort();
    all_user_ids.dedup();

    let user_map: std::collections::HashMap<Uuid, users::Model> = if all_user_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(all_user_ids))
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect()
    };

    for (work_order, symptom) in models_with_related {
        let product = load_zeus_product_model(work_order.product_id).await;

        let customer_user = user_map.get(&work_order.customer_id).cloned();
        let technician_user = work_order.technician_id.and_then(|tid| user_map.get(&tid).cloned());

        enriched_models.push((work_order, product, symptom, customer_user, technician_user));
    }

    let (data, meta) = list_svc::decide_list(enriched_models, &lookup_tables, &pagination, total_records);

    if let Some(mut conn) = conn_opt {
        if let Ok(cached_val) = serde_json::to_string(&(&data, &meta)) {
            let _: () = conn.set_ex(&full_cache_key, cached_val, 300).await.unwrap_or_default();
        }
    }

    Ok(axum::Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)))
}

pub(crate) async fn get_cached_technician_stats(
    db: &DatabaseConnection,
    valkey_client: &Option<Arc<ValkeyClient>>,
    technician_id: Uuid,
    closed_status_ids: &[i32],
) -> Result<TechnicianStatsSnapshot, AppError> {
    let cache_key = format!("cache:technician_stats:{}", technician_id);

    if let Some(client) = valkey_client.as_ref() {
        if let Ok(mut conn) = client.get_connection().await {
            if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
                if let Ok(snapshot) = serde_json::from_str::<TechnicianStatsSnapshot>(&cached_json) {
                    return Ok(snapshot);
                }
            }
        }
    }

    let work_order_rows = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::TechnicianId.eq(technician_id))
        .all(db)
        .await?;

    // Active jobs: work orders whose status is not in the closed set
    let active_jobs = work_order_rows
        .iter()
        .filter(|wo| !closed_status_ids.contains(&wo.work_order_status_id))
        .count() as i64;

    let total_work_orders = work_order_rows.len() as i64;
    let work_order_ids: Vec<Uuid> = work_order_rows.into_iter().map(|wo| wo.id).collect();

    let mut rating_sum = 0i64;
    let mut rating_count = 0i64;
    if !work_order_ids.is_empty() {
        let ratings = work_order_ratings::Entity::find()
            .filter(work_order_ratings::Column::WorkOrderId.is_in(work_order_ids))
            .all(db)
            .await?;

        for rating in ratings {
            rating_sum += i64::from(rating.rating);
            rating_count += 1;
        }
    }

    let snapshot = crate::services::v1::work_orders::technician_stats::decide_technician_stats(TechnicianStatsInput {
        total_work_orders,
        active_jobs,
        rating_sum,
        rating_count,
    });

    if let Some(client) = valkey_client.as_ref() {
        if let Ok(mut conn) = client.get_connection().await {
            if let Ok(cached_val) = serde_json::to_string(&snapshot) {
                let _: () = conn.set_ex(&cache_key, cached_val, 300).await.unwrap_or_default();
            }
        }
    }

    Ok(snapshot)
}

pub(crate) async fn refresh_technician_stats_cache(
    db: &DatabaseConnection,
    valkey_client: &Option<Arc<ValkeyClient>>,
    technician_id: Uuid,
    closed_status_ids: &[i32],
) {
    let _ = get_cached_technician_stats(db, valkey_client, technician_id, closed_status_ids).await;
}

/// Periodically clean up unassigned work orders that have exceeded the allowed wait window.

/// Cron-triggered cleanup: cancels all pending unassigned work orders that have exceeded
/// the threshold defined in the policy and notifies the customer.
pub async fn run_cleanup(
    db: &DatabaseConnection,
    luts: &LookupTables,
    valkey_client: Option<Arc<ValkeyClient>>,
    rabbitmq_opt: &Option<std::sync::Arc<lapin::Connection>>,
) -> Result<(), anyhow::Error> {
    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;

    // Get threshold from policy cache (LUT)
    let threshold_hours: i64 = luts.policies.get("unassigned_cleanup_threshold_hours").and_then(|v| v.parse().ok()).unwrap_or(3);

    let pending_status_id = *luts.work_order_statuses_by_name.get("Pending").unwrap_or_else(|| luts.work_order_statuses_by_name.get("Pending assignment").unwrap());
    let closed_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap();

    let now = Utc::now();
    let threshold_window = now + chrono::Duration::hours(threshold_hours);

    // Find WOs that are still pending assignment and whose appointment is within the threshold window (or already past)
    let target_wos = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::WorkOrderStatusId.eq(pending_status_id))
        .filter(work_orders_ent::Column::TechnicianId.is_null())
        .filter(work_orders_ent::Column::Appointment.lte(threshold_window))
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
                // Write-through cache: store full WorkOrderDetails in cache and bump list generation
                write_through_work_order_cache(db, valkey_client.clone(), luts, wo.id).await;
                info!("Cancelled unassigned WO {} successfully", wo.work_order_number);
                if let Some(rmq) = rabbitmq_opt.as_ref() {
                    let cust = users::Entity::find_by_id(wo.customer_id).one(db).await.unwrap_or_default();
                    if let Some(c) = cust {
                        let _ = crate::services::v1::core::email_service::send_email(
                            rmq,
                            &c.email,
                            "Work Order Cancelled",
                            &format!("Dear {},\n\nYour work order {} has been cancelled because your appointment is approaching and we could not assign a technician in time. We apologize for the inconvenience.", c.full_name, wo.work_order_number),
                        ).await;
                    }
                }
            }
            Err(e) => error!("Cleanup failed for WO {}: {}", wo.work_order_number, e),
        }
    }
    Ok(())
}

/// Attempt to automatically assign a single work order to a suitable technician.

pub(crate) async fn try_auto_assign_single(
    state: &AppState,
    db: Arc<DatabaseConnection>,
    wo: work_orders_ent::Model,
) -> bool {
    // Deduplication guard: if already assigned (by the other path), skip.
    if wo.technician_id.is_some() {
        tracing::info!("WO {} already has a technician — skipping auto-assign (already assigned)", wo.work_order_number);
        return true;
    }

    let luts = &state.lookup_tables;
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
    let agendas = match work_orders_ent::Entity::find().filter(work_orders_ent::Column::DeletedAt.is_null()).filter(work_orders_ent::Column::TechnicianId.is_in(tech_ids)).filter(work_orders_ent::Column::WorkOrderStatusId.ne(done_status_id)).all(db.as_ref()).await {
        Ok(a) => a, Err(e) => { tracing::warn!("Failed to load agendas: {}", e); return false; }
    };
    let mut technician_agendas: HashMap<Uuid, Vec<work_orders_ent::Model>> = HashMap::new();
    for a in agendas { if let Some(tid) = a.technician_id { technician_agendas.entry(tid).or_default().push(a); } }

    let effect = match crate::services::v1::work_orders::auto_assign::decide_auto_assign(wo.clone(), technicians, technician_agendas, policies, assigned_status_id, done_status_id, system_user_id) {
        Ok(Some(eff)) => eff,
        Ok(None) => { tracing::info!("Auto-assign: no suitable tech for WO {} — admin needed", wo.work_order_number); return false; }
        Err(e) => { tracing::error!("Auto-assign failed for WO {}: {}", wo.work_order_number, e); return false; }
    };
    let assigned_tech_id = effect.work_order_model.technician_id.clone().unwrap().unwrap();

    if let Err(e) = db.transaction::<_, (), anyhow::Error>(|txn| Box::pin(async move {
        effect.work_order_model.update(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        effect.state_history_model.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    })).await { tracing::error!("Auto-assign tx failed for WO {}: {}", wo.work_order_number, e); return false; }

    tracing::info!("Auto-assigned WO {} to {}", wo.work_order_number, assigned_tech_id);

    // Send push + in-app notifications to both technician and customer
    // (mongodb needed — we borrow from the state passed via the closure)
    let cust = users::Entity::find_by_id(wo.customer_id).one(db.as_ref()).await.unwrap_or_default();
    let tech = users::Entity::find_by_id(assigned_tech_id).one(db.as_ref()).await.unwrap_or_default();
    if let (Some(c), Some(t)) = (cust.as_ref(), tech.as_ref()) {
        let notification_data = serde_json::json!({
            "workOrderId": wo.id,
            "workOrderNumber": wo.work_order_number,
            "technicianName": t.full_name,
            "appointment": wo.appointment.to_string(),
        });

        // Notify technician
        let _ = crate::handlers::v1::notifications::send_notification::send_notification(
            state.mongodb.as_ref(),
            state.valkey.clone(),
            db.as_ref(),
            t.id,
            "work_order_assigned",
            "New Work Order Assigned",
            &format!("You have been assigned to work order {}", wo.work_order_number),
            notification_data.clone(),
        ).await;

        // Notify customer
        let _ = crate::handlers::v1::notifications::send_notification::send_notification(
            state.mongodb.as_ref(),
            state.valkey.clone(),
            db.as_ref(),
            c.id,
            "work_order_assigned",
            "Work Order Assigned",
            &format!("Your work order {} has been assigned to technician {}", wo.work_order_number, t.full_name),
            notification_data,
        ).await;
    }

    if let Some(rmq) = state.rabbitmq.as_ref() {
        if let (Some(c), Some(t)) = (cust.as_ref(), tech.as_ref()) {
            let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(rmq, &state.templates, &c.email, &c.full_name, &wo.work_order_number, &t.full_name, &wo.appointment.to_string()).await;
        }
    }
    // Auto-create 1-on-1 chat room between technician and customer
    if let (Some(ref t), Some(ref c)) = (&tech, &cust) {
        let _ = assign::ensure_chat_room(db.as_ref(), t.id, c.id, wo.id).await;
    }
    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    write_through_work_order_cache(db.as_ref(), state.valkey.clone(), &state.lookup_tables, wo.id).await;
    true
}
