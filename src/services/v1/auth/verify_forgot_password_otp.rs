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
        #[case] lua_result: i32,
        #[case] expected_result: &str,
    ) {
        let result = decide_verify_forgot_password_otp(
            lua_result,
            "test@example.com".to_string(),
            "token".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let effect = result.unwrap();
                assert_eq!(effect.email, "test@example.com");
                assert_eq!(effect.reset_token, "token");
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
