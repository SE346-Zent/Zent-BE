use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, TransactionTrait, ActiveModelTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::pause_request::PauseWorkOrderRequest;
use crate::model::responses::base::ApiResponse;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/pause", request_body = PauseWorkOrderRequest,
    responses(
        (status = 200, description = "Work order paused successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn pause(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PauseWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    let in_prog_status_id = *luts.work_order_statuses_by_name.get("InProg")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'InProg' status missing")))?;
    let paused_status_id = *luts.work_order_statuses_by_name.get("Paused")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Paused' status missing")))?;

    let effect = crate::services::v1::work_orders::pause::decide_pause_work_order(
        work_order,
        payload.reason,
        payload.explanation,
        in_prog_status_id,
        paused_status_id,
        auth.user.id,
    )?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order.update(txn).await?;
        effect.audit.insert(txn).await?;
        effect.state_history.insert(txn).await?;
        Ok(())
    }))
    .await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    Ok(Json(ApiResponse::message_only(200, "Work order paused successfully")))
}
