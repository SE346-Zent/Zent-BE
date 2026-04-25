use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    Json, Router, routing::post,
};
use std::net::SocketAddr;
use validator::Validate;
use crate::{
    core::{
        errors::{AppError, ErrorResponse},
        state::AppState,
    },
    model::{
        requests::auth::{
            user_login_request::UserLoginRequest,
            user_registration_request::UserRegistrationRequest,
            verify_otp_request::VerifyOtpRequest,
            resend_otp_request::ResendOtpRequest,
        },
        responses::{
            auth::login_response::LoginResponseData,
            base::{ApiResponse, MessageOnlyResponse},
        },
    },
    services::v1::auth::{login_service, register_service, verify_otp_service, resend_otp_service, refresh_token_service},
};
use crate::model::requests::auth::refresh_token_request::RefreshTokenRequest;

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh-token",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = ApiResponse<LoginResponseData>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn refresh_token_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    if let Err(errors) = payload.validate() {
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let result = refresh_token_service::perform_refresh(
        state.db.clone(),
        state.valkey.clone(),
        state.access_token_ttl,
        state.session_ttl,
        state.encoding_key.clone(),
        payload
    ).await?;
    
    Ok(Json(result))
}

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
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UserLoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    let ip_address = headers
        .get("X-Real-IP")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let result = login_service::perform_login(
        state.db.clone(),
        state.valkey.clone(),
        state.access_token_ttl,
        state.session_ttl,
        state.encoding_key.clone(),
        payload,
        ip_address
    ).await?;
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
    State(state): State<AppState>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if let Err(errors) = payload.validate() {
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let result = register_service::perform_register(
        state.db.clone(), 
        state.valkey.clone(), 
        state.rabbitmq.clone(), 
        payload
    ).await?;
    
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-otp",
    request_body = VerifyOtpRequest,
    responses(
        (status = 200, description = "Account verified successfully", body = MessageOnlyResponse),
        (status = 400, description = "Invalid OTP", body = ErrorResponse),
        (status = 403, description = "Too many failed attempts", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn verify_otp_handler(
    State(state): State<AppState>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if let Err(errors) = payload.validate() {
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let rabbitmq = state.rabbitmq.clone().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("RabbitMQ is not initialized"))
    })?;

    let result = verify_otp_service::perform_verify_otp(
        state.db.clone(), 
        state.valkey.clone().unwrap(), 
        rabbitmq, 
        payload
    ).await?;
    
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/resend-otp",
    request_body = ResendOtpRequest,
    responses(
        (status = 200, description = "OTP resent successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn resend_otp_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResendOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if let Err(errors) = payload.validate() {
        let err_msg = errors.to_string();
        return Err(AppError::BadRequest(err_msg));
    }

    let rabbitmq = state.rabbitmq.clone().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("RabbitMQ is not initialized"))
    })?;

    let result = resend_otp_service::perform_resend_otp(
        state.db.clone(), 
        state.valkey.clone().unwrap(), 
        rabbitmq, 
        payload
    ).await?;
    
    Ok(Json(result))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/resend-otp", post(resend_otp_handler))
        .route("/refresh-token", post(refresh_token_handler))
}
