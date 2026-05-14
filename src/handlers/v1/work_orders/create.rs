use axum::{extract::State, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::errors::ErrorResponse;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::core::config::AppConfig;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::create_work_order_request::CreateWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::create_response::WorkOrderResponseData;
use crate::services::v1::work_orders::create as create_svc;
use crate::entities::{products, work_orders as work_orders_ent};
use serde_json::json;
use redis::AsyncCommands;

use super::IDEMPOTENCY_PENDING;

#[utoipa::path(
    post, path = "/api/v1/work_orders", request_body = CreateWorkOrderRequest,
    responses(
        (status = 201, description = "Work order created successfully", body = ApiResponse<WorkOrderResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateWorkOrderRequest>,
) -> Result<Json<ApiResponse<WorkOrderResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let cfg = AppConfig::get();

    let idempotency_key = headers.get("X-Idempotency-Key").and_then(|v| v.to_str().ok());
    let mut conn_opt = None;
    let mut cache_key_opt: Option<String> = None;

    if let (Some(client), Some(key)) = (valkey_client.as_ref(), idempotency_key) {
        let mut conn = client.get_connection().await?;
        let cache_key = format!("idempotency:work_order:{}", key);
        let hashes = client.get_script_hashes();
        let script_sha = hashes.get("check_idempotency")
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("check_idempotency lua script missing")))?;
        let poll_delay = std::time::Duration::from_millis(cfg.idempotency_poll_delay_ms);
        let mut claimed = false;

        for _ in 0..=cfg.idempotency_poll_retries {
            let check_result: Option<String> = redis::cmd("EVALSHA")
                .arg(script_sha).arg(1).arg(&cache_key).arg(IDEMPOTENCY_PENDING).arg(cfg.idempotency_claim_ttl_seconds)
                .query_async(&mut conn).await?;
            match check_result {
                None => { claimed = true; break; }
                Some(val) if val == IDEMPOTENCY_PENDING => { tokio::time::sleep(poll_delay).await; }
                Some(json_str) => {
                    let cached_val: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| AppError::Internal(e.into()))?;

                    // cached_val is {"payload": <full request body>, "response": <response>}
                    // Compare the current request body directly against the cached payload
                    let current_payload = serde_json::to_value(&payload).map_err(|e| AppError::Internal(e.into()))?;
                    if current_payload == cached_val["payload"] {
                        // Same payload — genuine retry (e.g. network timeout), return cached response
                        let response: WorkOrderResponseData = serde_json::from_value(cached_val["response"].clone()).map_err(|e| AppError::Internal(e.into()))?;
                        return Ok(Json(ApiResponse::success(201, "Work order created successfully", response)));
                    }

                    // Different payload — the same idempotency key was used with a
                    // different request body. Reject to prevent silent overwrites.
                    return Err(AppError::Conflict(format!(
                        "Idempotency key '{}' was already used with a different request body",
                        key
                    )));
                }
            }
        }
        if !claimed { return Err(AppError::Conflict("A concurrent request with this idempotency key is still in progress".to_string())); }
        cache_key_opt = Some(cache_key);
        conn_opt = Some(conn);
    }

    if !products::Entity::find_by_id(payload.product_id).one(db.as_ref()).await?.is_some() {
        return Err(AppError::NotFound(format!("Product with ID {} not found", payload.product_id)));
    }
    if let Some(ref_id) = payload.reference_ticket_id {
        if work_orders_ent::Entity::find().filter(work_orders_ent::Column::Id.eq(ref_id)).filter(work_orders_ent::Column::CustomerId.eq(auth.user.id)).one(db.as_ref()).await?.is_none() {
            return Err(AppError::BadRequest(format!("Reference Work Order with ID {} not found", ref_id)));
        }
    }

    let pending_status_id = luts.work_order_statuses_by_name.get("Pending").copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;
    let effect = create_svc::decide_create_work_order(payload.clone(), auth.user.id, pending_status_id)?;

    let wo_model = db.transaction::<_, work_orders_ent::Model, AppError>(|txn| Box::pin(async move {
        let wo = effect.work_order.insert(txn).await?;
        effect.state_history.insert(txn).await?;
        Ok(wo)
    })).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), wo_model.id).await;

    // Publish to MQ for asynchronous auto-assignment
    if let Some(rmq) = rabbitmq.as_ref() {
        let producer = crate::infrastructure::mq::work_order::WorkOrderProducer::new(Some(rmq.clone()));
        let payload = serde_json::json!({ "id": wo_model.id });
        if let Ok(payload_bytes) = serde_json::to_vec(&payload) {
            let _ = producer.publish_created(&payload_bytes).await;
        }
    }

    let status_text = "Pending assignment".to_string();

    let response = WorkOrderResponseData { id: wo_model.id, work_order_number: wo_model.work_order_number, status: status_text };

    if let (Some(mut conn), Some(cache_key)) = (conn_opt, cache_key_opt) {
        let _: () = conn.set_ex(&cache_key, json!({"payload":payload,"response":response}).to_string(), cfg.idempotency_final_ttl_seconds).await?;
    }

    Ok(Json(ApiResponse::success(201, "Work order created successfully", response)))
}
