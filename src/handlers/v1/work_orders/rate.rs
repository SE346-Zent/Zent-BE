use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, QueryFilter, ColumnTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::rate_request::RateWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use redis::AsyncCommands;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/rate", request_body = RateWorkOrderRequest,
    responses(
        (status = 200, description = "Rating submitted successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict - rating already exists", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn rate(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<RateWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Fetch the work order (cache-backed)
    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    let closed_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    // Atomic check-and-insert inside a transaction to prevent concurrent duplicate ratings
    let customer_id = auth.user.id;
    let rating_val = payload.rating;
    let comment_val = payload.comment;
    let effect = crate::services::v1::work_orders::rate::decide_rate_work_order(
        work_order.clone(),
        customer_id,
        closed_status_id,
        rating_val,
        comment_val,
        false, // duplicate check done atomically inside transaction
    )?;

    let insert_result = db.transaction::<_, _, AppError>(|txn| {
        let rating_model = effect.rating_model.clone();
        let id_check = id;
        Box::pin(async move {
            // Re-check inside transaction for atomicity
            let exists = crate::entities::work_order_ratings::Entity::find()
                .filter(crate::entities::work_order_ratings::Column::WorkOrderId.eq(id_check))
                .one(txn)
                .await?
                .is_some();
            if exists {
                return Err(AppError::Conflict("A rating has already been submitted for this work order".to_string()));
            }
            rating_model.insert(txn).await?;
            Ok(())
        })
    }).await;

    match insert_result {
        Ok(()) => {},
        Err(sea_orm::TransactionError::Connection(db_err)) => {
            return Err(AppError::Internal(anyhow::anyhow!("Database error: {}", db_err)));
        }
        Err(sea_orm::TransactionError::Transaction(AppError::Conflict(msg))) => {
            return Err(AppError::Conflict(msg));
        }
        Err(sea_orm::TransactionError::Transaction(e)) => {
            return Err(e);
        }
    }

    // Write-through cache update for work order itself if needed (though work order did not change, let's keep consistency)
    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), id).await;

    // Cache increment logic in Valkey for the technician
    if let Some(tech_id) = work_order.technician_id {
        if let Some(client) = valkey_client.as_ref() {
            if let Ok(mut conn) = client.get_connection().await {
                let cache_key = format!("ratings:tech:{}", tech_id);
                let exists: bool = conn.exists(&cache_key).await.unwrap_or(false);
                if exists {
                    // Cache exists, increment this specific rating
                    let field = payload.rating.to_string();
                    let _: Result<(), redis::RedisError> = conn.hincr(&cache_key, &field, 1).await;
                }
            }
        }
    }

    Ok(Json(ApiResponse::message_only(200, "Rating submitted successfully")))
}
