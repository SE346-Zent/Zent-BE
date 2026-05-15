use axum::{extract::State, Json};
use std::sync::Arc;
use validator::Validate;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::services::v1::auth::verify_forgot_password_otp;
use crate::model::requests::auth::verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest;
use crate::model::responses::base::ApiResponse;
use redis::AsyncCommands;

use crate::model::responses::auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData;

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
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(payload): Json<VerifyForgotPasswordOtpRequest>,
) -> Result<Json<ApiResponse<VerifyForgotPasswordOtpResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Verification service temporarily unavailable. Please try again later.".to_string()))?;
    let mut conn = client.get_connection().await?;
    let script_hashes = client.get_script_hashes();
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("forgot_password_verification:{}", payload.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash).arg(1).arg(&valkey_key).arg(&payload.otp_code)
        .query_async(&mut conn).await?;

    let reset_token = uuid::Uuid::new_v4().to_string();
    let effect = verify_forgot_password_otp::decide_verify_forgot_password_otp(result, payload.email, reset_token)?;

    conn.set_ex::<_, _, ()>(&effect.reset_token_key, &effect.email, effect.ttl_seconds).await?;

    Ok(Json(ApiResponse::success(200, "Verified", VerifyForgotPasswordOtpResponseData { reset_token: effect.reset_token })))
}
