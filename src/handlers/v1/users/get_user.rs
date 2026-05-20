use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::responses::base::ApiResponse,
    model::responses::users::UserResponseData,
    services::v1::users::get_user,
};

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "users",
    responses(
        (status = 200, description = "Retrieve user successful", body = ApiResponse<UserResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn get_user_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    let target_user = users::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let effect = get_user::decide_get_user(current_user, target_user)?;

    Ok(Json(ApiResponse::success(200, "Retrieve user successful", effect.response_data)))
}
