use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::cancel_request::CancelWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::users;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/cancel", request_body = CancelWorkOrderRequest,
    responses(
        (status = 200, description = "Work order cancelled successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_opt): State<Option<Arc<lapin::Connection>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CancelWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Fetch the work order
    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    let closed_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let cancel_window_hours: i64 = luts.policies.get("customer_cancel_window_hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);

    let effect = crate::services::v1::work_orders::cancel::decide_cancel_work_order(
        work_order.clone(),
        closed_status_id,
        auth.user.id,
        cancel_window_hours,
    )?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order.update(txn).await?;
        effect.state_history.insert(txn).await?;
        Ok(())
    }))
    .await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Write-through cache update
    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    // Send cancellation email to the customer
    if let Some(rmq) = rabbitmq_opt.as_ref() {
        let cust = users::Entity::find_by_id(work_order.customer_id).one(db.as_ref()).await.unwrap_or_default();
        if let Some(c) = cust {
            let _ = crate::services::v1::core::email_service::send_email(
                rmq,
                &c.email,
                "Work Order Cancelled",
                &format!("Dear {},\n\nYour work order {} has been cancelled successfully. We hope to serve you again in the future.", c.full_name, work_order.work_order_number),
            ).await;
        }
    }

    Ok(Json(ApiResponse::message_only(200, "Work order cancelled successfully")))
}
