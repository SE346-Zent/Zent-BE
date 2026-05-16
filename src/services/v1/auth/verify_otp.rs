use crate::{
    core::errors::AppError,
    entities::users,
};

/// Plain struct representing the side-effects that need to be persisted
pub struct VerifyOtpEffect {
    pub user_id: uuid::Uuid,
    pub active_status_id: i32,
    pub email: String,
    pub full_name: String,
}

/// Pure logic to decide the outcome of an OTP verification attempt.
pub fn decide_verify_otp(
    lua_result: i32,
    user_model: Option<&users::Model>,
    active_status_id: i32,
) -> Result<VerifyOtpEffect, AppError> {
    match lua_result {
        1 => {
            match user_model {
                Some(user) => {
                    Ok(VerifyOtpEffect {
                        user_id: user.id,
                        active_status_id,
                        email: user.email.clone(),
                        full_name: user.full_name.clone(),
                    })
                }
                None => Err(AppError::NotFound("User not found".to_string())),
            }
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
        #[case] lua_result: i32,
        #[case] provide_user: bool,
        #[case] expected_result: &str,
        mock_user: users::Model,
    ) {
        let user_ref = if provide_user { Some(&mock_user) } else { None };
        let result = decide_verify_otp(lua_result, user_ref, 2);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let effect = result.unwrap();
                assert_eq!(effect.user_id, mock_user.id);
                assert_eq!(effect.active_status_id, 2);
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
