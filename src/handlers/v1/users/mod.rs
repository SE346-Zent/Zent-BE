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
    model::responses::users::{UserResponseData, UserListResponseData},
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

pub async fn get_me_handler(
    _auth: AuthUser,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    unimplemented!()
}

pub async fn update_me_handler(
    _db: State<Arc<DatabaseConnection>>,
    _auth: AuthUser,
    _payload: Json<ProfileUpdateRequest>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    unimplemented!()
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
