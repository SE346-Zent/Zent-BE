use axum::Json;
use crate::{
    core::errors::AppError,
    extractor::auth_user::AuthUser,
    model::responses::base::ApiResponse,
    model::responses::users::MeResponseData,
    services::v1::users::get_me,
};

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    responses(
        (status = 200, description = "Retrieve profile successful", body = ApiResponse<MeResponseData>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn get_me_handler(
    AuthUser { user, .. }: AuthUser,
) -> Result<Json<ApiResponse<MeResponseData>>, AppError> {
    let effect = get_me::decide_get_me(user);
    Ok(Json(ApiResponse::success(200, "Retrieve profile successful", effect.response_data)))
}
