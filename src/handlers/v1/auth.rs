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
            forgot_password_request::ForgotPasswordRequest,
            verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
            reset_password_request::ResetPasswordRequest,
        },
        responses::{
            auth::login_response::LoginResponseData,
            auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
            base::{ApiResponse, MessageOnlyResponse},
        },
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "OTP sent successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn forgot_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.forgot_password(payload).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-forgot-password-otp",
    request_body = VerifyForgotPasswordOtpRequest,
    responses(
        (status = 200, description = "OTP verified successfully", body = ApiResponse<VerifyForgotPasswordOtpResponseData>),
        (status = 400, description = "Invalid OTP", body = ErrorResponse),
        (status = 403, description = "Too many failed attempts", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn verify_forgot_password_otp_handler(
    State(state): State<AppState>,
    Json(payload): Json<VerifyForgotPasswordOtpRequest>,
) -> Result<Json<ApiResponse<VerifyForgotPasswordOtpResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.verify_forgot_password_otp(payload).await?;
    Ok(Json(result))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successfully", body = MessageOnlyResponse),
        (status = 400, description = "Invalid token", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn reset_password_handler(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.reset_password(payload).await?;
    Ok(Json(result))
}

use crate::model::requests::auth::refresh_token_request::RefreshTokenRequest;

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh-token",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn refresh_token_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.refresh_token(payload).await?;
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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let ip_address = headers
        .get("X-Real-IP")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let result = state.auth_service.login(payload, ip_address).await?;
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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.register(payload).await?;
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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.verify_otp(payload).await?;
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
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let result = state.auth_service.resend_otp(payload).await?;
    Ok(Json(result))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/resend-otp", post(resend_otp_handler))
        .route("/refresh-token", post(refresh_token_handler))
        .route("/forgot-password", post(forgot_password_handler))
        .route("/verify-forgot-password-otp", post(verify_forgot_password_otp_handler))
        .route("/reset-password", post(reset_password_handler))
}
