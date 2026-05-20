use crate::core::errors::AppError;

/// Represents the calculated results and side-effects of a successful forgot password OTP verification.
pub struct VerifyForgotPasswordOtpEffect {
    /// The generated unique reset token to be returned to the client.
    pub password_reset_token: String,
    /// The cache key under which the reset token will be stored in Valkey.
    pub reset_token_cache_key: String,
    /// The email address of the user who verified the OTP.
    pub user_email: String,
    /// The time-to-live for the reset token in seconds.
    pub token_ttl_seconds: u64,
}

/// Determine the outcome of a forgot password OTP verification based on the Lua script result.
///
/// This pure function maps the integer codes returned by the Valkey Lua script 
/// to appropriate application-level results and side-effects.
///
/// # Arguments
/// * `lua_verification_result` - The integer result code from the `verify_otp` Lua script.
/// * `user_email` - The email address of the user attempting verification.
/// * `new_password_reset_token` - A newly generated unique token for the subsequent reset step.
///
/// # Returns
/// A result containing the `VerifyForgotPasswordOtpEffect` on success, or an `AppError`.
pub fn decide_verify_forgot_password_otp(
    lua_verification_result: i32,
    user_email: String,
    new_password_reset_token: String,
) -> Result<VerifyForgotPasswordOtpEffect, AppError> {
    match lua_verification_result {
        1 => {
            Ok(VerifyForgotPasswordOtpEffect {
                password_reset_token: new_password_reset_token.clone(),
                reset_token_cache_key: format!("password_reset_token:{}", new_password_reset_token),
                user_email,
                token_ttl_seconds: 900,
            })
        }
        -1 => Err(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => Err(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => Err(AppError::Forbidden("Too many attempts".to_string())),
        _ => Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", lua_verification_result))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1, "Ok")] // Success
    #[case(-1, "BadRequest")] // Expired
    #[case(-2, "BadRequest")] // Invalid
    #[case(-3, "Forbidden")] // Too many attempts
    #[case(99, "Internal")] // Internal
    fn test_decide_verify_forgot_password_otp_exhaustive(
        #[case] lua_verification_result: i32,
        #[case] expected_result: &str,
    ) {
        let result = decide_verify_forgot_password_otp(
            lua_verification_result,
            "test@example.com".to_string(),
            "token".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let verify_effect = result.unwrap();
                assert_eq!(verify_effect.user_email, "test@example.com");
                assert_eq!(verify_effect.password_reset_token, "token");
                assert_eq!(verify_effect.reset_token_cache_key, "password_reset_token:token");
                assert_eq!(verify_effect.token_ttl_seconds, 900);
            }
            "BadRequest" => {
                assert!(matches!(result, Err(AppError::BadRequest(_))));
            }
            "Forbidden" => {
                assert!(matches!(result, Err(AppError::Forbidden(_))));
            }
            "Internal" => {
                assert!(matches!(result, Err(AppError::Internal(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
