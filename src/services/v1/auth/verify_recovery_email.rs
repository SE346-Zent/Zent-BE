use crate::core::errors::AppError;

/// Pure logic: determine the outcome of a recovery email OTP verification.
///
/// Maps the integer codes from the Valkey Lua script to application-level results.
pub fn decide_verify_recovery_email(
    lua_result: i32,
) -> Result<(), AppError> {
    match lua_result {
        1 => Ok(()),
        -1 => Err(AppError::BadRequest("OTP has expired".to_string())),
        -2 => Err(AppError::BadRequest("Invalid OTP code".to_string())),
        -3 => Err(AppError::Forbidden("Too many attempts. Please request a new OTP".to_string())),
        _ => Err(AppError::Internal(anyhow::anyhow!("Unexpected verification result: {}", lua_result))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(1, true)]
    #[case(-1, false)]
    #[case(-2, false)]
    #[case(-3, false)]
    #[case(99, false)]
    fn test_decide_verify_recovery_email(#[case] lua_result: i32, #[case] should_succeed: bool) {
        let result = decide_verify_recovery_email(lua_result);
        assert_eq!(result.is_ok(), should_succeed);
    }
}
