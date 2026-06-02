use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the calculated results and side-effects of a successful password reset request.
pub struct ResetPasswordEffect {
    /// The unique identifier of the user whose password is being reset.
    pub user_id: uuid::Uuid,
    /// The newly generated Argon2 hash of the user's new password.
    pub new_password_hash: String,
    /// The cache key of the reset token that should be invalidated.
    pub reset_token_cache_key: String,
}

/// Determine the outcome of a password reset attempt by validating the new password against the current one.
///
/// This pure function ensures that users do not reset their password to the exact same 
/// value they are currently using, enhancing security.
///
/// # Arguments
/// * `user_record` - The database model of the user whose password is being reset.
/// * `is_new_password_same_as_current` - Boolean indicating if the new password matches the current one.
/// * `new_password_hash` - The newly generated hash for the user's new password.
/// * `reset_token_cache_key` - The cache key associated with the used reset token.
///
/// # Returns
/// A result containing the `ResetPasswordEffect` on success, or a `BadRequest` error if the password is the same.
pub fn decide_reset_password(
    user_record: &users::Model,
    is_new_password_same_as_current: bool,
    new_password_hash: String,
    reset_token_cache_key: String,
) -> Result<ResetPasswordEffect, AppError> {
    if is_new_password_same_as_current {
        tracing::warn!(
            reason = "NewPasswordSameAsCurrent",
            user_id = %user_record.id,
            email = %user_record.email,
            "Reset password failed: new password is the same as the current password"
        );
        return Err(AppError::BadRequest("New password cannot be the same as current".to_string()));
    }

    tracing::info!(
        user_id = %user_record.id,
        email = %user_record.email,
        "Reset password decided successfully"
    );

    Ok(ResetPasswordEffect {
        user_id: user_record.id,
        new_password_hash,
        reset_token_cache_key,
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
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(false, "Ok")] // Different password
    #[case(true, "BadRequest")] // Same password
    fn test_decide_reset_password_exhaustive(
        #[case] is_new_password_same_as_current: bool,
        #[case] expected_result: &str,
        mock_user: users::Model,
    ) {
        let result = decide_reset_password(
            &mock_user,
            is_new_password_same_as_current,
            "new_password_hash".to_string(),
            "reset_token_cache_key".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let reset_effect = result.unwrap();
                assert_eq!(reset_effect.user_id, mock_user.id);
                assert_eq!(reset_effect.new_password_hash, "new_password_hash");
                assert_eq!(reset_effect.reset_token_cache_key, "reset_token_cache_key");
            }
            "BadRequest" => {
                assert!(matches!(result, Err(AppError::BadRequest(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
