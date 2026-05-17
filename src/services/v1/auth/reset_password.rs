use crate::{
    core::errors::AppError,
    entities::users,
};

/// Plain struct representing the side-effects that need to be persisted
pub struct ResetPasswordEffect {
    pub user_id: uuid::Uuid,
    pub new_hash: String,
    pub reset_token_key: String,
}

/// Pure logic to decide the outcome of a password reset attempt.
pub fn decide_reset_password(
    user_model: &users::Model,
    is_same_password: bool,
    new_hash: String,
    reset_token_key: String,
) -> Result<ResetPasswordEffect, AppError> {
    if is_same_password {
        return Err(AppError::BadRequest("New password cannot be the same as current".to_string()));
    }

    Ok(ResetPasswordEffect {
        user_id: user_model.id,
        new_hash,
        reset_token_key,
    })
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
            password_hash: "old_hash".to_string(),
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
    #[case(false, "Ok")] // Different password
    #[case(true, "BadRequest")] // Same password
    fn test_decide_reset_password_exhaustive(
        #[case] is_same_password: bool,
        #[case] expected_result: &str,
        mock_user: users::Model,
    ) {
        let result = decide_reset_password(
            &mock_user,
            is_same_password,
            "new_hash".to_string(),
            "token_key".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let effect = result.unwrap();
                assert_eq!(effect.user_id, mock_user.id);
                assert_eq!(effect.new_hash, "new_hash");
                assert_eq!(effect.reset_token_key, "token_key");
            }
            "BadRequest" => {
                assert!(matches!(result, Err(AppError::BadRequest(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
