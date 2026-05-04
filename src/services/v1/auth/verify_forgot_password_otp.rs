use crate::core::errors::AppError;

/// Plain struct representing the side-effects that need to be persisted
pub struct VerifyForgotPasswordOtpEffect {
    pub reset_token: String,
    pub reset_token_key: String,
    pub email: String,
    pub ttl_seconds: u64,
}

/// Pure logic to decide the outcome of a forgot password OTP verification attempt.
pub fn decide_verify_forgot_password_otp(
    lua_result: i32,
    email: String,
    reset_token: String,
) -> Result<VerifyForgotPasswordOtpEffect, AppError> {
    match lua_result {
        1 => {
            Ok(VerifyForgotPasswordOtpEffect {
                reset_token: reset_token.clone(),
                reset_token_key: format!("password_reset_token:{}", reset_token),
                email,
                ttl_seconds: 900,
            })
        }
        -1 => Err(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => Err(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => Err(AppError::Forbidden("Too many attempts".to_string())),
        _ => Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", lua_result))),
    }
}
