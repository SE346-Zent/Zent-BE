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
/// Handle requests to verify the OTP sent during the forgotten password flow.
///
/// This handler executes a Valkey Lua script to verify the OTP against the
/// cached value. If successful, it generates a password reset token and
/// stores it in Valkey for the final reset step.
///
/// # Arguments
/// * `valkey_client` - Optional shared Valkey client for OTP verification and token storage.
/// * `verify_payload` - The request containing the user's email and the OTP code.
///
/// # Returns
/// A result containing the successful `ApiResponse` with the reset token, or an `AppError`.
pub async fn verify_forgot_password_otp_handler(
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(verify_payload): Json<VerifyForgotPasswordOtpRequest>,
) -> Result<Json<ApiResponse<VerifyForgotPasswordOtpResponseData>>, AppError> {
    verify_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Verification service temporarily unavailable. Please try again later.".to_string()))?;
    let mut valkey_conn = client.get_connection().await?;
    let script_hashes = client.get_script_hashes();
    let verify_otp_script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_otp_key = format!("forgot_password_verification:{}", verify_payload.email);
    let lua_result: i32 = redis::cmd("EVALSHA")
        .arg(verify_otp_script_hash).arg(1).arg(&valkey_otp_key).arg(&verify_payload.otp_code)
        .query_async(&mut valkey_conn).await?;

    let new_reset_token = uuid::Uuid::new_v4().to_string();
    let verify_effect = verify_forgot_password_otp::decide_verify_forgot_password_otp(lua_result, verify_payload.email, new_reset_token)?;

    valkey_conn.set_ex::<_, _, ()>(&verify_effect.reset_token_cache_key, &verify_effect.user_email, verify_effect.token_ttl_seconds).await?;

    Ok(Json(ApiResponse::success(200, "Verified", VerifyForgotPasswordOtpResponseData { reset_token: verify_effect.password_reset_token })))
}
