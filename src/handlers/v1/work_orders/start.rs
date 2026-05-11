use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::work_orders::start_request::StartWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::work_orders as work_orders_ent;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/start", request_body = StartWorkOrderRequest,
    responses(
        (status = 200, description = "Work order started successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"), (status = 500, description = "Internal Server Error")
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
    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;
    let in_prog_id = *luts.work_order_statuses_by_name.get("InProg").ok_or_else(|| AppError::Internal(anyhow::anyhow!("In Progress status not found")))?;

    let effect = crate::services::v1::work_orders::start::decide_start(payload, wo, auth.user.id, in_prog_id, &luts.policies).await?;
    db.transaction::<_, (), AppError>(|txn| Box::pin(async move { effect.work_order.update(txn).await?; effect.state_history.insert(txn).await?; Ok(()) }))
        .await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;
    Ok(Json(ApiResponse::message_only(200, "Work order started successfully")))
}
