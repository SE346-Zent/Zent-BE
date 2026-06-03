use axum::{extract::State, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::errors::ErrorResponse;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::core::config::AppConfig;
use crate::core::state::AppState;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::metrics;
use crate::model::requests::work_orders::create_work_order_request::CreateWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::work_orders::create_response::WorkOrderResponseData;
use crate::services::v1::work_orders::create as create_svc;
use crate::entities::{work_orders as work_orders_ent, registered_devices};
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
    State(state): State<AppState>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
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
                    let current_payload = serde_json::to_value(&payload).map_err(|e| AppError::Internal(e.into()))?;
                    if current_payload == cached_val["payload"] {
                        let response: WorkOrderResponseData = serde_json::from_value(cached_val["response"].clone()).map_err(|e| AppError::Internal(e.into()))?;
                        return Ok(Json(ApiResponse::success(201, "Work order created successfully", response)));
                    }
                    return Err(AppError::Conflict(format!(
                        "This idempotency key was already used with a different request"
                    )));
                }
            }
        }
        if !claimed { return Err(AppError::Conflict("Another request with this idempotency key is still in progress".to_string())); }
        cache_key_opt = Some(cache_key);
        conn_opt = Some(conn);
    }

    state.zeus_client.get_product(payload.product_id).await?;
    if let Some(ref_id) = payload.reference_ticket_id {
        if work_orders_ent::Entity::find().filter(work_orders_ent::Column::Id.eq(ref_id)).filter(work_orders_ent::Column::DeletedAt.is_null()).filter(work_orders_ent::Column::CustomerId.eq(auth.user.id)).one(db.as_ref()).await?.is_none() {
            return Err(AppError::BadRequest("Reference work order not found".to_string()));
        }
    }

    // Check for registered device data to use as fallback for email and phone number
    let mut payload = payload;
    let registered_device = registered_devices::Entity::find()
        .filter(registered_devices::Column::CustomerId.eq(auth.user.id))
        .filter(registered_devices::Column::ProductId.eq(payload.product_id))
        .one(db.as_ref())
        .await?;

    if let Some(ref device) = registered_device {
        // Use registered device email as fallback if not provided
        if payload.email.is_none() || payload.email.as_ref().map_or(true, |e| e.is_empty()) {
            payload.email = Some(device.email.clone());
        }
        // Use registered device phone as fallback if not provided
        if payload.phone_number.is_none() || payload.phone_number.as_ref().map_or(true, |p| p.is_empty()) {
            payload.phone_number = Some(device.mobile_phone.clone());
        }
    }

    let pending_status_id = luts.work_order_statuses_by_name.get("Pending").copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;
    let effect = create_svc::decide_create_work_order(payload.clone(), auth.user.id, pending_status_id, &luts.policies)?;

    let wo_model = db.transaction::<_, work_orders_ent::Model, AppError>(|txn| Box::pin(async move {
        let wo = effect.work_order_model.insert(txn).await?;
        effect.state_history_model.insert(txn).await?;
        Ok(wo)
    })).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Write-through cache
    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), wo_model.id).await;

    // --- Direct auto-assign ---
    // Spawn as a background task so it doesn't block the HTTP response.
    {
        let state_clone = state.clone();
        let db_clone = db.clone();
        let wo_id = wo_model.id;
        tokio::spawn(async move {
            let wo_fresh = match work_orders_ent::Entity::find_by_id(wo_id).one(db_clone.as_ref()).await {
                Ok(Some(w)) => w,
                Ok(None) => { tracing::warn!("WO {} not found for direct auto-assign", wo_id); return; }
                Err(e) => { tracing::warn!("Failed to fetch WO {} for direct auto-assign: {}", wo_id, e); return; }
            };
            tracing::info!("Running auto-assign directly for WO {}", wo_id);
            let success = super::try_auto_assign_single(&state_clone, db_clone, wo_fresh).await;
            if !success {
                tracing::info!("Direct auto-assign did not complete for WO {} — admin may need to assign manually", wo_id);
            }
        });
    }

    // --- MQ publish (commented out — direct call is the sole path) ---
    // {
    //     let producer = crate::infrastructure::mq::work_order::WorkOrderProducer::new();
    //     let mq_payload = serde_json::json!({ "id": wo_model.id });
    //     if let Ok(payload_bytes) = serde_json::to_vec(&mq_payload) {
    //         tracing::info!("Publishing WO {} to MQ for auto-assignment", wo_model.id);
    //         if let Err(e) = producer.publish_created(&payload_bytes).await {
    //             tracing::warn!("Failed to publish WO {} to MQ: {}", wo_model.id, e);
    //         }
    //     }
    // }

    let response = WorkOrderResponseData {
        id: wo_model.id,
        work_order_number: wo_model.work_order_number,
        status: "Pending assignment".to_string(),
    };

    if let (Some(mut conn), Some(cache_key)) = (conn_opt, cache_key_opt) {
        let _: () = conn.set_ex(&cache_key, json!({"payload":payload,"response":response}).to_string(), cfg.idempotency_final_ttl_seconds).await?;
    }

    metrics::init().wo_created_total.add(1, &[]);

    Ok(Json(ApiResponse::success(201, "Work order created successfully", response)))
}
