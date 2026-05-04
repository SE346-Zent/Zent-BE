use axum::{
    extract::{State},
    http::{HeaderMap, StatusCode},
    Json, Extension,
};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::{
    requests::work_orders::create_work_order_request::CreateWorkOrderRequest,
    responses::{
        work_orders::work_order_response::WorkOrderResponseData,
    },
};
use crate::services::v1::work_orders::create;
use redis::AsyncCommands;
use serde_json::json;

use crate::entities::{products, work_orders as work_orders_ent};

#[utoipa::path(
    post,
    path = "/api/v1/work_orders",
    request_body = CreateWorkOrderRequest,
    responses(
        (status = 201, description = "Work order created successfully", body = WorkOrderResponseData),
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
) -> Result<(StatusCode, Json<WorkOrderResponseData>), AppError> {
    // 1. Idempotency Check
    let idempotency_key = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    let mut conn_opt = None;

    if let (Some(client), Some(key)) = (valkey_client.as_ref(), idempotency_key) {
        let mut conn = client.get_connection();
        let cache_key = format!("idempotency:work_order:{}", key);
        
        let cached: Option<String> = conn.get(&cache_key).await?;
        if let Some(json_str) = cached {
            let cached_val: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| AppError::Internal(e.into()))?;
            
            // Check if payload matches (use a simple hash or string comparison)
            let payload_json = serde_json::to_value(&payload).map_err(|e| AppError::Internal(e.into()))?;
            if cached_val["payload"] != payload_json {
                return Err(AppError::Conflict("Idempotency key reused with different payload".to_string()));
            }

            let response: WorkOrderResponseData = serde_json::from_value(cached_val["response"].clone())
                .map_err(|e| AppError::Internal(e.into()))?;
            return Ok((StatusCode::CREATED, Json(response)));
        }
        conn_opt = Some(conn);
    }

    // 2. Data Integrity Checks (Verify Foreign Keys to avoid 500s)
    // Check if Product exists
    let product_exists = products::Entity::find_by_id(payload.product_id)
        .one(db.as_ref())
        .await?
        .is_some();
    if !product_exists {
        return Err(AppError::NotFound(format!("Product with ID {} not found", payload.product_id)));
    }

    // Check if Reference Ticket exists (if provided)
    if let Some(ref_id) = payload.reference_ticket_id {
        let ref_exists = work_orders_ent::Entity::find_by_id(ref_id)
            .one(db.as_ref())
            .await?
            .is_some();
        if !ref_exists {
            return Err(AppError::BadRequest(format!("Reference Work Order with ID {} not found", ref_id)));
        }
    }

    // 3. Prepare Data
    let pending_status_id = luts.work_order_statuses_by_name.get("Pending")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;

    // 4. Decision Logic (Pure)
    let effect = create::decide_create_work_order(payload.clone(), auth.user.id, pending_status_id)?;

    // 5. Execution (I/O)
    let wo_model = effect.work_order.insert(db.as_ref()).await?;

    let response = WorkOrderResponseData {
        id: wo_model.id,
        work_order_number: wo_model.work_order_number,
        status: "Pending assignment".to_string(), // Requirement: start with this status name in response
    };

    // 5. Store Idempotency
    if let (Some(mut conn), Some(key)) = (conn_opt, idempotency_key) {
        let cache_key = format!("idempotency:work_order:{}", key);
        let cache_val = json!({
            "payload": payload,
            "response": response
        }).to_string();
        let _: () = conn.set_ex(&cache_key, cache_val, 3600).await?;
    }

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn get_details(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

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
