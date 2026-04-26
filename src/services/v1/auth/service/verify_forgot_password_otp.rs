use std::collections::HashMap;
use crate::{
    core::errors::AppError,
    model::{
        responses::base::ApiResponse,
        responses::auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
    },
    model::requests::auth::verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
};
use redis::AsyncCommands;
use uuid::Uuid;

pub async fn handle_verify_forgot_password_otp(
    valkey: Option<redis::aio::MultiplexedConnection>,
    script_hashes: &HashMap<String, String>,
    req: VerifyForgotPasswordOtpRequest,
) -> Result<ApiResponse<VerifyForgotPasswordOtpResponseData>, AppError> {
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("forgot_password_verification:{}", req.email);
    let result: i32 = redis::cmd("EVALSHA").arg(script_hash).arg(1).arg(&valkey_key).arg(&req.otp_code)
        .query_async(&mut conn).await?;

    match result {
        1 => {}
        -1 => return Err(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => return Err(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => return Err(AppError::Forbidden("Too many attempts".to_string())),
        _ => return Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", result))),
    }

    let reset_token = Uuid::new_v4().to_string();
    let reset_token_key = format!("password_reset_token:{}", reset_token);
    conn.set_ex::<_, _, ()>(&reset_token_key, &req.email, 900).await?;

    Ok(ApiResponse::success(200, "Verified", VerifyForgotPasswordOtpResponseData { reset_token }))
}
