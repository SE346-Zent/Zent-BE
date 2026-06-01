use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait};
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::requests::users::UserStatusUpdateRequest,
    model::responses::base::ApiResponse,
    services::v1::users::update_status,
};

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/status",
    tag = "users",
    request_body = UserStatusUpdateRequest,
    responses(
        (status = 200, description = "Update status successful"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn update_user_status_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UserStatusUpdateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let target_user = users::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("User account not found".to_string()))?;

    let effect = update_status::decide_can_update_status(current_user, target_user, payload)?;

    effect.user_active_model.update(db.as_ref()).await?;

    Ok(Json(ApiResponse::success(200, "Update status successful", ())))
}
