use axum::{
    extract::{ConnectInfo, State},
    Json, Router, routing::post,
};
use std::net::SocketAddr;
use validator::Validate;
use crate::{
    errors::{AppError, ErrorResponse},
    model::{
        requests::auth::{
            user_login_request::UserLoginRequest,
            user_registration_request::UserRegistrationRequest,
        },
        responses::{
            auth::login_response::LoginResponseData,
            base::{ApiResponse, MessageOnlyResponse},
        },
    },
    services::v1::auth::{login_service, register_service},
    state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
};
use sea_orm::DatabaseConnection;
use jsonwebtoken::EncodingKey;

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = UserLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn login_handler(
    State(db): State<DatabaseConnection>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(session_ttl): State<SessionDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UserLoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    let result = login_service::perform_login(db, access_token_ttl, session_ttl, encoding_key, payload, addr).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = UserRegistrationRequest,
    responses(
        (status = 201, description = "Registration successful", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 409, description = "Conflict Validation Error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn register_handler(
    State(db): State<DatabaseConnection>,
    State(rabbitmq): State<std::sync::Arc<lapin::Connection>>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if let Err(errors) = payload.validate() {
        // Collect errors into a message
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let result = register_service::perform_register(db, rabbitmq, payload).await?;
    Ok(Json(result))
}

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
}
