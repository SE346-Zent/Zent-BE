use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::{
    core::errors::AppError,
    extractor::auth_user::AuthUser,
    model::requests::users::ProfileUpdateRequest,
    model::responses::base::ApiResponse,
    model::responses::users::MeResponseData,
    services::v1::users::update_me,
};

#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "users",
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Update profile successful", body = ApiResponse<MeResponseData>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn update_me_handler(
    State(_db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<ProfileUpdateRequest>,
) -> Result<Json<ApiResponse<MeResponseData>>, AppError> {
    let effect = update_me::decide_update_me(user, payload)?;
    Ok(Json(ApiResponse::success(200, "Update profile successful", effect.response_data)))
}
