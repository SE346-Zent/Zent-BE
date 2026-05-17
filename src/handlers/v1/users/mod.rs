use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, post, put},
    Json, Router, middleware,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    core::state::AppState,
    entities::roles::Role,
    extractor::auth_user::AuthUser,
    extractor::role_check::require_role,
    model::requests::users::{ProfileUpdateRequest, UserCreateRequest, UserStatusUpdateRequest},
    model::responses::base::ApiResponse,
    model::responses::users::{UserResponseData, UserListResponseData, MeResponseData},
    services::v1::users::{self, UserListQuery},
};

pub fn router(state: AppState) -> Router<AppState> {
    let generic_routes = Router::new()
        .route("/me", get(get_me_handler))
        .route("/me", put(update_me_handler))
        .route("/me/close", post(close_account_handler));

    let admin_routes = Router::new()
        .route("/", get(list_users_handler))
        .route("/", post(create_user_handler))
        .route("/{id}", get(get_user_handler))
        .route("/{id}/status", patch(update_user_status_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    Router::new()
        .merge(generic_routes)
        .merge(admin_routes)
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
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
    let data = users::get_me(user).await?;
    Ok(Json(ApiResponse::success(200, "Retrieve profile successful", data)))
}

#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Update profile successful", body = ApiResponse<MeResponseData>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn update_me_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<ProfileUpdateRequest>,
) -> Result<Json<ApiResponse<MeResponseData>>, AppError> {
    let data = users::update_me(&db, user, payload).await?;
    Ok(Json(ApiResponse::success(200, "Update profile successful", data)))
}

pub async fn close_account_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}

pub async fn list_users_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
    _query: Query<UserListQuery>,
) -> Result<Json<ApiResponse<UserListResponseData>>, AppError> {
    unimplemented!()
}

pub async fn get_user_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
    _id: Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    unimplemented!()
}

pub async fn create_user_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
    _payload: Json<UserCreateRequest>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    unimplemented!()
}

pub async fn update_user_status_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
    _id: Path<Uuid>,
    _payload: Json<UserStatusUpdateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    unimplemented!()
}
