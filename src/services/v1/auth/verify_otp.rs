use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the calculated results and side-effects of a successful registration OTP verification.
pub struct VerifyOtpEffect {
    /// The unique identifier of the user who has been verified.
    pub verified_user_id: uuid::Uuid,
    /// The database ID representing the 'Active' account status.
    pub target_active_status_id: i32,
    /// The email address of the verified user.
    pub user_email: String,
    /// The full name of the verified user.
    pub user_full_name: String,
}

/// Determine the outcome of a registration OTP verification based on the Lua script result and user existence.
///
/// This pure function maps the integer codes returned by the Valkey Lua script
/// to appropriate application-level results, and ensures the user record exists
/// for the verified email.
///
/// # Arguments
/// * `lua_verification_result` - The integer result code from the `verify_otp` Lua script.
/// * `user_record` - An optional database model of the user matching the email.
/// * `target_active_status_id` - The database ID to be assigned to the user upon success.
///
/// # Returns
/// A result containing the `VerifyOtpEffect` on success, or an `AppError`.
pub fn decide_verify_otp(
    lua_verification_result: i32,
    user_record: Option<&users::Model>,
    target_active_status_id: i32,
) -> Result<VerifyOtpEffect, AppError> {
    match lua_verification_result {
        1 => {
            match user_record {
                Some(user) => {
                    Ok(VerifyOtpEffect {
                        verified_user_id: user.id,
                        target_active_status_id,
                        user_email: user.email.clone(),
                        user_full_name: user.full_name.clone(),
                    })
                }
                None => Err(AppError::NotFound("User account not found".to_string())),
            }
        }
        -1 => Err(AppError::BadRequest("OTP has expired".to_string())),
        -2 => Err(AppError::BadRequest("Invalid OTP code".to_string())),
        -3 => Err(AppError::Forbidden("Too many attempts. Please request a new OTP".to_string())),
        _ => Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", lua_verification_result))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::users;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_user() -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "123".to_string(),
            account_status: 1,
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            recovery_email: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(1, true, "Ok")] // Success
    #[case(1, false, "NotFound")] // User missing
    #[case(-1, false, "BadRequest")] // Expired
    #[case(-2, false, "BadRequest")] // Invalid
    #[case(-3, false, "Forbidden")] // Too many attempts
    #[case(99, false, "Internal")] // Internal
    fn test_decide_verify_otp_exhaustive(
        #[case] lua_verification_result: i32,
        #[case] provide_user_record: bool,
        #[case] expected_result: &str,
        mock_user: users::Model,
    ) {
        let user_record_ref = if provide_user_record { Some(&mock_user) } else { None };
        let result = decide_verify_otp(lua_verification_result, user_record_ref, 2);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let verify_effect = result.unwrap();
                assert_eq!(verify_effect.verified_user_id, mock_user.id);
                assert_eq!(verify_effect.target_active_status_id, 2);
            }
            "NotFound" => {
                assert!(matches!(result, Err(AppError::NotFound(_))));
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
