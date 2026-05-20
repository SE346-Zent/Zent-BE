use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait};
use validator::Validate;
use crate::{
    core::errors::AppError,
    extractor::auth_user::AuthUser,
    model::requests::auth::change_password_request::ChangePasswordRequest,
    model::responses::base::ApiResponse,
    services::v1::auth::change_password,
    utils::hasher,
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/change-password",
    tag = "auth",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed successful"),
        (status = 400, description = "Bad Request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn change_password_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Verify old password
    let is_valid = hasher::verify_password(payload.old_password, user.password_hash.clone()).await?;

    // Pure logic: validate + prepare ActiveModel
    let effect = change_password::decide_change_password(user, is_valid, String::new())?;

    // Hash new password
    let hashed = hasher::hash_password(payload.new_password).await?;
    let mut model = effect.user_active_model;
    model.password_hash = sea_orm::Set(hashed);

    model.update(db.as_ref()).await?;

    Ok(Json(ApiResponse::success(200, "Password changed successful", ())))
}
