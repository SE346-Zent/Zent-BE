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

use crate::entities::{products, work_orders as work_orders_ent, work_order_symptoms};
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
    let wo_model = effect.work_order.insert(db.as_ref()).await?;

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

pub async fn assign(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn schedule(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn start(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn refuse(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn cancel(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn complete(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn history(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn add_parts(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn approve_refusal(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn deny_refusal(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
