use axum::{
    extract::{State},
    http::{HeaderMap, StatusCode},
    Json, Extension,
};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use validator::Validate;
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
use crate::core::config::AppConfig;

/// Sentinel value stored during the idempotency claim window.
/// If a concurrent reader sees this, the original request is still in-flight.
const IDEMPOTENCY_PENDING: &str = "__PENDING__";

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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let cfg = AppConfig::get();

    // ── 1. Atomic Idempotency Claim ─────────────────────────────────────
    // Uses a Lua script to atomically check and claim the idempotency slot.
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
            // Atomic claim via Lua script:
            // Returns `None` if it successfully claimed (key didn't exist).
            // Returns `Some(value)` if the key already existed.
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
                    // Successfully claimed the idempotency key!
                    claimed = true;
                    break;
                }
                Some(val) if val == IDEMPOTENCY_PENDING => {
                    // Still in-flight, wait and retry.
                    tokio::time::sleep(poll_delay).await;
                }
                Some(json_str) => {
                    // We got the finalized JSON response from a previous request.
                    let cached_val: serde_json::Value =
                        serde_json::from_str(&json_str)
                            .map_err(|e| AppError::Internal(e.into()))?;

                    // Verify payload matches to detect key reuse with different data.
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
                    return Ok((StatusCode::CREATED, Json(response)));
                }
            }
        }

        if !claimed {
            // Exhausted retries and still PENDING
            return Err(AppError::Conflict(
                "A concurrent request with this idempotency key is still in progress".to_string(),
            ));
        }

        cache_key_opt = Some(cache_key);
        conn_opt = Some(conn);
    }

    // ── 2. Data Integrity Checks ────────────────────────────────────────
    // Check if Product exists
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

    // Check if Reference Ticket exists AND belongs to the same customer.
    // Constraining by customer_id prevents linking to another tenant's work order.
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

    let response = WorkOrderResponseData {
        id: wo_model.id,
        work_order_number: wo_model.work_order_number,
        status: "Pending assignment".to_string(),
    };

    // ── 6. Finalise Idempotency ─────────────────────────────────────────
    // Overwrite the PENDING sentinel with the real response + longer TTL.
    if let (Some(mut conn), Some(cache_key)) = (conn_opt, cache_key_opt) {
        let cache_val = json!({
            "payload": payload,
            "response": response
        })
        .to_string();
        let _: () = conn.set_ex(&cache_key, cache_val, cfg.idempotency_final_ttl_seconds).await?;
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
