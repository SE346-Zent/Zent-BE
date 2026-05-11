use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::add_parts_request::AddPartsRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::work_orders as work_orders_ent;

#[utoipa::path(
    post, path = "/api/v1/inventory/work_orders/{id}/parts", request_body = AddPartsRequest,
    responses(
        (status = 200, description = "Parts added successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"), (status = 500, description = "Internal Server Error")
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
    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;
    let effect = crate::services::v1::inventory::add_parts::decide_add_parts(payload, wo, auth.user.id)?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.new_part_form.insert(txn).await?;
        for img in effect.images { img.insert(txn).await?; }
        for link in effect.image_links { link.insert(txn).await?; }
        Ok(())
    })).await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    Ok(Json(ApiResponse::message_only(200, "Parts added successfully")))
}
