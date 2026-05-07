use axum::{
    extract::{State, Query, Path},
    http::{HeaderMap, StatusCode},
    Json, Extension,
};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *, prelude::Uuid};
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::{
    requests::work_orders::{
        create_work_order_request::CreateWorkOrderRequest,
        list_query::WorkOrderQuery,
        assign_request::AssignWorkOrderRequest,
        complete_request::CompleteWorkOrderRequest,
        refuse_request::RefuseWorkOrderRequest,
        start_request::StartWorkOrderRequest,
        approve_refusal_request::ApproveRefusalRequest,
        add_parts_request::AddPartsRequest,
        refuse_request::RefuseWorkOrderMultipart,
        complete_request::CompleteWorkOrderMultipart,
    },
    requests::pagination::PaginationRequest,
    responses::{
        pagination::PaginationResponse,
        work_orders::{
            create_response::WorkOrderResponseData,
            list_response::WorkOrderListItem,
            details_response::WorkOrderDetails,
        },
    },
};
use crate::services::v1::work_orders::{create, list as list_svc, get_details as get_svc};
use redis::AsyncCommands;
use serde_json::json;

use crate::entities::{products, work_orders as work_orders_ent, work_order_symptoms, work_order_image_links, users};
use crate::entities::work_orders as work_orders;
use sea_orm::TransactionTrait;
use crate::core::config::AppConfig;

use crate::model::responses::base::ApiResponse;

/// Sentinel value stored during the idempotency claim window.
/// If a concurrent reader sees this, the original request is still in-flight.
const IDEMPOTENCY_PENDING: &str = "__PENDING__";

#[utoipa::path(
    post,
    path = "/api/v1/work_orders",
    request_body = CreateWorkOrderRequest,
    responses(
        (status = 201, description = "Work order created successfully", body = ApiResponse<WorkOrderResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    headers: HeaderMap,
    Json(payload): Json<CreateWorkOrderRequest>,
) -> Result<Json<ApiResponse<WorkOrderResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let cfg = AppConfig::get();

    // ── 1. Atomic Idempotency Claim ─────────────────────────────────────
    let idempotency_key = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    let mut conn_opt = None;
    let mut cache_key_opt: Option<String> = None;

    if let (Some(client), Some(key)) = (valkey_client.as_ref(), idempotency_key) {
        let mut conn = client.get_connection();
        let cache_key = format!("idempotency:work_order:{}", key);

        let script_hashes = client.get_script_hashes();
        let script_sha = script_hashes.get("check_idempotency")
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("check_idempotency lua script missing")))?;

        let poll_delay = std::time::Duration::from_millis(cfg.idempotency_poll_delay_ms);
        let mut claimed = false;

        for _ in 0..=cfg.idempotency_poll_retries {
            let check_result: Option<String> = redis::cmd("EVALSHA")
                .arg(script_sha)
                .arg(1)
                .arg(&cache_key)
                .arg(IDEMPOTENCY_PENDING)
                .arg(cfg.idempotency_claim_ttl_seconds)
                .query_async(&mut conn)
                .await?;

            match check_result {
                None => {
                    claimed = true;
                    break;
                }
                Some(val) if val == IDEMPOTENCY_PENDING => {
                    tokio::time::sleep(poll_delay).await;
                }
                Some(json_str) => {
                    let cached_val: serde_json::Value =
                        serde_json::from_str(&json_str)
                            .map_err(|e| AppError::Internal(e.into()))?;

                    let payload_json = serde_json::to_value(&payload)
                        .map_err(|e| AppError::Internal(e.into()))?;
                    if cached_val["payload"] != payload_json {
                        return Err(AppError::Conflict(
                            "Idempotency key reused with different payload".to_string(),
                        ));
                    }

                    let response: WorkOrderResponseData =
                        serde_json::from_value(cached_val["response"].clone())
                            .map_err(|e| AppError::Internal(e.into()))?;
                    return Ok(Json(ApiResponse::success(201, "Work order created successfully", response)));
                }
            }
        }

        if !claimed {
            return Err(AppError::Conflict(
                "A concurrent request with this idempotency key is still in progress".to_string(),
            ));
        }

        cache_key_opt = Some(cache_key);
        conn_opt = Some(conn);
    }

    // ── 2. Data Integrity Checks ────────────────────────────────────────
    let product_exists = products::Entity::find_by_id(payload.product_id)
        .one(db.as_ref())
        .await?
        .is_some();
    if !product_exists {
        return Err(AppError::NotFound(format!(
            "Product with ID {} not found",
            payload.product_id
        )));
    }

    if let Some(ref_id) = payload.reference_ticket_id {
        let ref_wo = work_orders_ent::Entity::find()
            .filter(work_orders_ent::Column::Id.eq(ref_id))
            .filter(work_orders_ent::Column::CustomerId.eq(auth.user.id))
            .one(db.as_ref())
            .await?;
        if ref_wo.is_none() {
            return Err(AppError::BadRequest(format!(
                "Reference Work Order with ID {} not found",
                ref_id
            )));
        }
    }

    // ── 3. Prepare Data ─────────────────────────────────────────────────
    let pending_status_id = luts
        .work_order_statuses_by_name
        .get("Pending")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;

    // ── 4. Decision Logic (Pure) ────────────────────────────────────────
    let effect =
        create::decide_create_work_order(payload.clone(), auth.user.id, pending_status_id)?;

    // ── 5. Execution (I/O) ──────────────────────────────────────────────
    let wo_model = db.transaction::<_, work_orders_ent::Model, AppError>(|txn| {
        Box::pin(async move {
            let wo = effect.work_order.insert(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(wo)
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // ── 5.1 Invalidate List Caches (Namespace/Generation Invalidation) ──
    if let Some(client) = valkey_client.as_ref() {
        let mut conn = client.get_connection();
        let _: () = conn.incr("cache:work_orders:generation", 1).await.unwrap_or_default();
    }

    let response = WorkOrderResponseData {
        id: wo_model.id,
        work_order_number: wo_model.work_order_number,
        status: "Pending assignment".to_string(),
    };

    // ── 6. Finalise Idempotency ─────────────────────────────────────────
    if let (Some(mut conn), Some(cache_key)) = (conn_opt, cache_key_opt) {
        let cache_val = json!({
            "payload": payload,
            "response": response
        })
        .to_string();
        let _: () = conn.set_ex(&cache_key, cache_val, cfg.idempotency_final_ttl_seconds).await?;
    }

    Ok(Json(ApiResponse::success(201, "Work order created successfully", response)))
}

#[utoipa::path(
    get,
    path = "/api/v1/work_orders",
    params(WorkOrderQuery),
    responses(
        (status = 200, description = "List of work orders based on user role", body = ApiResponse<Vec<WorkOrderListItem>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Query(query): Query<WorkOrderQuery>,
) -> Result<Json<ApiResponse<Vec<WorkOrderListItem>>>, AppError> {
    
    // 1. Compare role in parameter with role in Access Token
    if let Some(requested_role) = &query.role {
        if requested_role != &auth.role.name {
            return Err(AppError::Forbidden(format!(
                "Requested context '{}' does not match your assigned role '{}'",
                requested_role, auth.role.name
            )));
        }
    }

    // 2. Resolve the security filters based on the role in the Access Token
    let mut resolved_province = query.province.clone();
    let mut resolved_tech_id = query.technician_id;
    let mut resolved_customer_id = None;
    let cache_key_prefix;

    match auth.role.name.as_str() {
        "SuperAdmin" => {
            // SuperAdmins can see everything and use any explicit query parameter
            cache_key_prefix = format!("superadmin:p:{:?}:t:{:?}", resolved_province, resolved_tech_id);
        }
        "Admin" => {
            // Admins are locked to their own province. Override any requested province.
            let admin_province = auth.user.province.clone().ok_or_else(|| {
                AppError::Forbidden("Admin profile is missing assigned province".to_string())
            })?;
            resolved_province = Some(admin_province.clone());
            cache_key_prefix = format!("admin_geo:{}:t:{:?}", admin_province, resolved_tech_id);
        }
        "Technician" => {
            // Technicians can only see their own assigned work orders. Override requested tech_id.
            resolved_tech_id = Some(auth.user.id);
            cache_key_prefix = format!("tech:{}:p:{:?}", auth.user.id, resolved_province);
        }
        "Customer" => {
            // Customers can only see their own created work orders.
            resolved_customer_id = Some(auth.user.id);
            resolved_tech_id = None;
            resolved_province = None;
            cache_key_prefix = format!("customer:{}", auth.user.id);
        }
        _ => return Err(AppError::Forbidden("Role not recognized in unified handler".to_string())),
    }

    fetch_paginated_work_orders(
        db, 
        valkey_client, 
        lookup_tables, 
        query.pagination, 
        &cache_key_prefix, 
        resolved_tech_id, 
        resolved_province,
        resolved_customer_id
    ).await
}

async fn fetch_paginated_work_orders(
    db: Arc<DatabaseConnection>,
    valkey_client: Option<Arc<ValkeyClient>>,
    lookup_tables: Arc<LookupTables>,
    pagination: PaginationRequest,
    cache_key_prefix: &str,
    technician_id: Option<Uuid>,
    province_filter: Option<String>,
    customer_id: Option<Uuid>,
) -> Result<Json<ApiResponse<Vec<WorkOrderListItem>>>, AppError> {
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
                return Ok(Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)));
            }
        }
        conn_opt = Some(conn);
    }

    let mut query = work_orders_ent::Entity::find();

    if let Some(tech_id) = technician_id {
        query = query.filter(work_orders_ent::Column::TechnicianId.eq(tech_id));
    }

    if let Some(province) = province_filter {
        query = query.filter(work_orders_ent::Column::Province.eq(province));
    }

    if let Some(cust_id) = customer_id {
        query = query.filter(work_orders_ent::Column::CustomerId.eq(cust_id));
    }

    let paginator = query.clone()
        .order_by_desc(work_orders_ent::Column::CreatedAt)
        .paginate(db.as_ref(), pagination.limit);

    let total_records = paginator.num_items().await?;
    
    let models_with_related = query
        .order_by_desc(work_orders_ent::Column::CreatedAt)
        .find_also_related(products::Entity)
        .find_also_related(work_order_symptoms::Entity)
        .offset((pagination.page - 1) * pagination.limit)
        .limit(pagination.limit)
        .all(db.as_ref())
        .await?;

    let (data, meta) = list_svc::decide_list(models_with_related, &lookup_tables, &pagination, total_records);

    if let Some(mut conn) = conn_opt {
        if let Ok(cached_val) = serde_json::to_string(&(&data, &meta)) {
            let _: () = conn.set_ex(&full_cache_key, cached_val, 300).await.unwrap_or_default();
        }
    }

    Ok(Json(ApiResponse::success_with_meta(200, "Work orders retrieved successfully", data, meta)))
}

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/refusal/approve",
    request_body = ApproveRefusalRequest,
    responses(
        (status = 200, description = "Refusal approved and work order reassigned", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order or technician not found"),
        (status = 409, description = "Technician schedule conflict"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn approve_refusal(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ApproveRefusalRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 1. Fetch data
    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // Check geofence admin
    if auth.user.province.as_ref() != Some(&work_order.province) {
        return Err(AppError::Forbidden("You can only manage work orders in your area".into()));
    }

    let reject_form_id = work_order.reject_form_id
        .ok_or_else(|| AppError::BadRequest("This work order does not have a rejection form".to_string()))?;

    let reject_form = crate::entities::work_order_reject_forms::Entity::find_by_id(reject_form_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Rejection form not found".to_string()))?;

    let technician = users::Entity::find_by_id(payload.technician_id)
        .filter(users::Column::RoleId.eq(3)) // Assuming Role::Technician = 3
        .filter(users::Column::Province.eq(work_order.province.clone()))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Technician not found or not in this area".to_string()))?;

    let technician_work_orders = work_orders::Entity::find()
        .filter(work_orders::Column::TechnicianId.eq(technician.id))
        .all(db.as_ref())
        .await?;

    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Assigned status not found")))?;
    let _in_progress_status_id = *luts.work_order_statuses_by_name.get("In Progress")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("In Progress status not found")))?;
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Closed status not found")))?;

    // 2. Decision Logic
    let effect = crate::services::v1::work_orders::approve_refusal::decide_approve_refusal(
        work_order,
        reject_form,
        technician,
        technician_work_orders,
        auth.user.id,
        assigned_status_id,
        done_status_id,
    )?;

    // 3. Execution
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.work_order.update(txn).await?;
            effect.reject_form.update(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Refusal approved and work order reassigned")))
}

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/refusal/deny",
    responses(
        (status = 200, description = "Refusal denied, status reverted", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn deny_refusal(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 1. Fetch data
    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // Check geofence admin
    if auth.user.province.as_ref() != Some(&work_order.province) {
        return Err(AppError::Forbidden("You can only manage work orders in your area".into()));
    }

    let reject_form_id = work_order.reject_form_id
        .ok_or_else(|| AppError::BadRequest("This work order does not have a rejection form".to_string()))?;

    let reject_form = crate::entities::work_order_reject_forms::Entity::find_by_id(reject_form_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Rejection form not found".to_string()))?;

    let rejected_status_id = *luts.work_order_statuses_by_name.get("Rejected")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Rejected status not found")))?;

    // 2. Decision Logic
    let effect = crate::services::v1::work_orders::deny_refusal::decide_deny_refusal(
        work_order,
        reject_form,
        auth.user.id,
        rejected_status_id,
    )?;

    // 3. Execution
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.work_order.update(txn).await?;
            effect.reject_form.update(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Refusal denied successfully")))
}

#[utoipa::path(
    get,
    path = "/api/v1/work_orders/{id}",
    responses(
        (status = 200, description = "Work order details", body = ApiResponse<WorkOrderDetails>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_details(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<WorkOrderDetails>>, AppError> {
    let mut conn_opt = None;
    let cache_key = format!("cache:work_order:{}", id);

    if let Some(client) = valkey_client.as_ref() {
        let mut conn = client.get_connection();
        if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(details) = serde_json::from_str::<WorkOrderDetails>(&cached_json) {
                if has_access_to_work_order(&auth, &details) {
                    return Ok(Json(ApiResponse::success(200, "Work order details retrieved successfully", details)));
                }
            }
        }
        conn_opt = Some(conn);
    }

    let result = work_orders_ent::Entity::find_by_id(id)
        .find_also_related(products::Entity)
        .find_also_related(work_order_symptoms::Entity)
        .one(db.as_ref())
        .await?;

    let (wo, product, symptom) = result.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let details = get_svc::decide_get_details(wo, product, symptom, &lookup_tables);

    if !has_access_to_work_order(&auth, &details) {
        return Err(AppError::Forbidden("You do not have permission to view this work order".to_string()));
    }

    if let Some(mut conn) = conn_opt {
        if let Ok(cached_val) = serde_json::to_string(&details) {
            let _: () = conn.set_ex(&cache_key, cached_val, 600).await.unwrap_or_default();
        }
    }

    Ok(Json(ApiResponse::success(200, "Work order details retrieved successfully", details)))
}

fn has_access_to_work_order(auth: &AuthUser, details: &WorkOrderDetails) -> bool {
    if auth.role.name == "SuperAdmin" || auth.role.name == "Admin" {
        return true;
    }
    
    if auth.user.id == details.customer_id {
        return true;
    }
    false
}

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/assign",
    request_body = AssignWorkOrderRequest,
    responses(
        (status = 200, description = "Work order assigned successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn assign(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(rabbitmq_opt): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AssignWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let work_order = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if auth.role.name == "Admin" {
        let admin_province = auth.user.province.as_ref()
            .ok_or_else(|| AppError::Forbidden("Admin has no province assigned".to_string()))?;
        let wo_province = &work_order.province;
        if admin_province != wo_province {
            return Err(AppError::Forbidden("Admin province does not match work order province".to_string()));
        }
    }

    let technician_work_orders = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::TechnicianId.eq(payload.technician_id))
        .all(db.as_ref())
        .await?;

    // Just use "Assigned" status from seeder
    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Assigned' status missing")))?;
    
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let effect = crate::services::v1::work_orders::assign::decide_assign_work_order(
        payload.clone(),
        work_order.clone(),
        technician_work_orders,
        &luts.policies,
        assigned_status_id,
        done_status_id,
        auth.user.id,
    )?;

    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.work_order.update(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Send email to customer
    if let Some(rabbitmq) = rabbitmq_opt.as_ref() {
        let technician = users::Entity::find_by_id(payload.technician_id)
            .one(db.as_ref())
            .await?;
        let customer = users::Entity::find_by_id(work_order.customer_id)
            .one(db.as_ref())
            .await?;
        
        if let (Some(tech), Some(cust)) = (technician, customer) {
            let tech_name = tech.full_name.clone();
            let cust_name = cust.full_name.clone();
            let appointment = work_order.appointment.to_string();
            
            let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(
                rabbitmq,
                &templates,
                &cust.email,
                &cust_name,
                &work_order.work_order_number,
                &tech_name,
                &appointment,
            ).await;
        }
    }

    Ok(Json(ApiResponse::success(200, "Work order assigned successfully", ())))
}

pub async fn schedule(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/start",
    request_body = StartWorkOrderRequest,
    responses(
        (status = 200, description = "Work order started successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn start(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<StartWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // 1. Fetch data
    let work_order = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    let in_progress_status_id = *luts.work_order_statuses_by_name.get("InProg")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("In Progress status not found")))?;

    // 2. Decision Logic
    let effect = crate::services::v1::work_orders::start::decide_start(
        payload,
        work_order,
        auth.user.id,
        in_progress_status_id,
        &luts.policies,
    ).await?;

    // 3. Execution
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.work_order.update(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Work order started successfully")))
}

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/refuse",
    request_body(content = RefuseWorkOrderMultipart, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Work order refusal submitted successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn refuse(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut reason = String::new();
    let mut explanation = String::new();
    let mut photos_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "photos" {
            let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
            let file_name = field.file_name().unwrap_or("photo.jpg").to_string();
            if let Ok(data) = field.bytes().await {
                photos_data.push((data, content_type, file_name));
            }
        } else if let Ok(text) = field.text().await {
            match name.as_str() {
                "reason" => reason = text,
                "explanation" => explanation = text,
                _ => {}
            }
        }
    }

    if reason.is_empty() {
        return Err(AppError::BadRequest("reason is required".to_string()));
    }
    
    if photos_data.len() > 5 {
        return Err(AppError::BadRequest("A maximum of 5 photos are allowed".to_string()));
    }

    let work_order = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if work_order.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let mut evidence_image_urls = Vec::new();
    for (data, ct, file_name) in photos_data {
        let extension = file_name.split('.').last().unwrap_or("jpg");
        let unique_name = format!(
            "refuse_{}_{}_{}.{}", 
            id, 
            chrono::Utc::now().timestamp(), 
            Uuid::new_v4(), 
            extension
        );
        crate::utils::oci::upload_object(&unique_name, data.to_vec(), &ct).await?;
        evidence_image_urls.push(unique_name);
    }

    let payload = RefuseWorkOrderRequest {
        reason,
        explanation,
        evidence_image_urls,
    };

    let refuse_in_review_status_id = *luts.work_order_statuses_by_name.get("Reject_InReview")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Reject_InReview' status missing")))?;

    let effect = crate::services::v1::work_orders::refuse::decide_refuse_work_order(
        payload,
        work_order,
        refuse_in_review_status_id,
        auth.user.id,
    )?;

    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.reject_form.insert(txn).await?;
            for img in effect.images {
                img.insert(txn).await?;
            }
            for link in effect.image_links {
                link.insert(txn).await?;
            }
            effect.work_order.update(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::success(200, "Work order refusal submitted successfully", ())))
}

pub async fn cancel(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/complete",
    request_body = CompleteWorkOrderRequest,
    responses(
        (status = 200, description = "Work order completed successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn complete(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CompleteWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let work_order = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if work_order.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // Geofencing check
    let target_location = crate::utils::geocoding::geocode_address(
        &work_order.address,
        &work_order.city,
        &work_order.province,
        &work_order.country,
    ).await?;

    let radius: f64 = luts.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000.0);
    
    let is_verified = crate::utils::geo::is_within_geofence(
        payload.latitude,
        payload.longitude,
        target_location.lat,
        target_location.lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let completed_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    // Fetch existing image links to validate required photos
    let existing_image_links = work_order_image_links::Entity::find()
        .filter(work_order_image_links::Column::WorkOrderId.eq(id))
        .all(db.as_ref())
        .await?;

    let effect = crate::services::v1::work_orders::complete::decide_complete_work_order(
        payload,
        work_order,
        existing_image_links,
        &luts.policies,
        completed_status_id,
        auth.user.id,
    )?;

    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.closing_form.insert(txn).await?;
            for img in effect.images {
                img.insert(txn).await?;
            }
            for link in effect.image_links {
                link.insert(txn).await?;
            }
            for pc in effect.part_changes {
                pc.insert(txn).await?;
            }
            for pu in effect.part_updates {
                pu.update(txn).await?;
            }
            if let Some(ot) = effect.overtime {
                ot.insert(txn).await?;
            }
            effect.work_order.update(txn).await?;
            effect.state_history.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::success(200, "Work order completed successfully", ())))
}

pub async fn history(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{id}/parts",
    request_body = AddPartsRequest,
    responses(
        (status = 200, description = "Parts added successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn add_parts(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddPartsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // 1. Fetch data
    let work_order = work_orders_ent::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // 2. Decision Logic
    let effect = crate::services::v1::work_orders::add_parts::decide_add_parts(
        payload,
        work_order,
        auth.user.id,
    )?;

    // 3. Execution
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.new_part_form.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Parts added successfully")))
}
