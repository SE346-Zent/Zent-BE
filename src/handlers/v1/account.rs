use axum::{
    extract::{Query, State},
    Json, Router, routing::get,
};
use sea_orm::DatabaseConnection;

use crate::{
    extractor::auth_user::AuthUser,
    model::{
        requests::account::profile_list_query::ProfileListQuery,
        responses::account::profile_response::{ProfileListResponse, ProfileResponse},
        responses::error::{AppError, ErrorResponse},
    },
    services::v1::account::profile_service::{get_profile_service, get_profiles_service},
};

#[utoipa::path(
    get,
    path = "/api/v1/account/profile",
    responses(
        (status = 200, description = "Retrieve Profile", body = ProfileResponse),
        (status = 401, description = "Unauthorized JWT Invalid", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_profile(
    State(db): State<DatabaseConnection>,
    auth: AuthUser,
) -> Result<Json<ProfileResponse>, AppError> {
    // Utilize the hydrated context!
    let result = get_profile_service(db, auth.user.id).await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/account/profiles",
    params(ProfileListQuery),
    responses(
        (status = 200, description = "Retrieve Profile List", body = ProfileListResponse),
        (status = 400, description = "Bad Request Constraints", body = ErrorResponse),
        (status = 401, description = "Unauthorized Executions", body = ErrorResponse),
        (status = 500, description = "Server Trapped Error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_profiles(
    State(db): State<DatabaseConnection>,
    Query(query): Query<ProfileListQuery>,
    _auth: AuthUser,
) -> Result<Json<ProfileListResponse>, AppError> {
    let result = get_profiles_service(db, query).await?;
    Ok(Json(result))
}

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profiles", get(get_profiles))
}
