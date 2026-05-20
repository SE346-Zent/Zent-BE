use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::{
    core::errors::AppError,
    extractor::auth_user::AuthUser,
    model::responses::base::ApiResponse,
    services::v1::users::close_account,
};

#[utoipa::path(
    post,
    path = "/api/v1/users/me/close",
    tag = "users",
    responses(
        (status = 200, description = "Account closed successful"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn close_account_handler(
    State(_db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
) -> Result<Json<ApiResponse<()>>, AppError> {
    close_account::decide_close_account(user)?;
    Ok(Json(ApiResponse::success(200, "Account closed successful", ())))
}
