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

/// Describes the outcome of a forgot password OTP verification attempt.
pub enum VerifyForgotPasswordOtpIntent {
    Success {
        reset_token: String,
        reset_token_key: String,
        email: String,
        ttl_seconds: u64,
    },
    Error(AppError),
}

/// Pure logic to decide the outcome of a forgot password OTP verification attempt.
pub fn decide_verify_forgot_password_otp(
    lua_result: i32,
    email: String,
) -> VerifyForgotPasswordOtpIntent {
    match lua_result {
        1 => {
            let reset_token = Uuid::new_v4().to_string();
            VerifyForgotPasswordOtpIntent::Success {
                reset_token: reset_token.clone(),
                reset_token_key: format!("password_reset_token:{}", reset_token),
                email,
                ttl_seconds: 900,
            }
        }
        -1 => VerifyForgotPasswordOtpIntent::Error(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => VerifyForgotPasswordOtpIntent::Error(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => VerifyForgotPasswordOtpIntent::Error(AppError::Forbidden("Too many attempts".to_string())),
        _ => VerifyForgotPasswordOtpIntent::Error(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", lua_result))),
    }
}

pub async fn handle_verify_forgot_password_otp(
    valkey: Option<redis::aio::MultiplexedConnection>,
    script_hashes: &HashMap<String, String>,
    req: VerifyForgotPasswordOtpRequest,
) -> Result<ApiResponse<VerifyForgotPasswordOtpResponseData>, AppError> {
    // 1. I/O: Interact with Valkey
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("forgot_password_verification:{}", req.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash)
        .arg(1)
        .arg(&valkey_key)
        .arg(&req.otp_code)
        .query_async(&mut conn)
        .await?;

    // 2. Decision Logic (Pure)
    let intent = decide_verify_forgot_password_otp(result, req.email);

    // 3. Execution (I/O)
    match intent {
        VerifyForgotPasswordOtpIntent::Success { reset_token, reset_token_key, email, ttl_seconds } => {
            conn.set_ex::<_, _, ()>(&reset_token_key, &email, ttl_seconds).await?;
            Ok(ApiResponse::success(200, "Verified", VerifyForgotPasswordOtpResponseData { reset_token }))
        }
        VerifyForgotPasswordOtpIntent::Error(err) => Err(err),
    }
}
