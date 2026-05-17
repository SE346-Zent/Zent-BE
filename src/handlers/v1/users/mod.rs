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

/// Initialize and return the Axum router for user management.
pub fn router(state: AppState) -> Router<AppState> {
    let generic_routes = Router::new()
        .route("/me", get(get_me_handler))
        .route("/me", put(update_me_handler))
        .route("/me/close", post(close_account_handler));

    let admin_only_routes = Router::new()
        .route("/", get(list_users_handler))
        .route("/", post(create_user_handler))
        .route("/{id}", get(get_user_handler))
        .route("/{id}/status", patch(update_user_status_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin, Role::SuperAdmin]),
        ));

    Router::new()
        .merge(generic_routes)
        .merge(admin_only_routes)
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
    let effect = users::get_me(user).await?;
    Ok(Json(ApiResponse::success(200, "Retrieve profile successful", effect.response_data)))
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
    let effect = users::update_me(&db, user, payload).await?;
    Ok(Json(ApiResponse::success(200, "Update profile successful", effect.response_data)))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/me/close",
    responses(
        (status = 200, description = "Account closed successful"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn close_account_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
) -> Result<Json<ApiResponse<()>>, AppError> {
    users::close_account(&db, user).await?;
    Ok(Json(ApiResponse::success(200, "Account closed successful", ())))
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(UserListQuery),
    responses(
        (status = 200, description = "List users successful", body = ApiResponse<UserListResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn list_users_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    Query(query): Query<UserListQuery>,
) -> Result<Json<ApiResponse<UserListResponseData>>, AppError> {
    let effect = users::list_users(&db, user, query).await?;
    Ok(Json(ApiResponse::success(200, "List users successful", UserListResponseData {
        users: effect.users.into_iter().map(|u| UserResponseData {
            id: u.id,
            role_id: u.role_id,
            full_name: u.full_name,
            email: u.email,
            phone: Some(u.phone_number),
            account_status_id: u.account_status,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }).collect(),
        total: effect.total,
    })))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
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
    AuthUser { user, .. }: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    let effect = users::get_user(&db, user, id).await?;
    Ok(Json(ApiResponse::success(200, "Retrieve user successful", effect.response_data)))
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = UserCreateRequest,
    responses(
        (status = 201, description = "User created successful", body = ApiResponse<UserResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn create_user_handler(
    State(db): State<Arc<DatabaseConnection>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<UserCreateRequest>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    let _data = users::create_user(&db, user, payload).await?;
    // Handler would normally convert the effect to a response
    unimplemented!()
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/status",
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
    AuthUser { user, .. }: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UserStatusUpdateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    users::update_user_status(&db, user, id, payload).await?;
    Ok(Json(ApiResponse::success(200, "Update status successful", ())))
}
