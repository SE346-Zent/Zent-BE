use crate::core::errors::AppError;
use crate::entities::users;

/// Pure logic: validate inputs and prepare the recovery email effect.
///
/// Checks:
/// - Password must be verified before calling this function.
/// - Recovery email must differ from the primary email.
/// - Recovery email must differ from the current recovery email (if already set).
pub fn decide_set_recovery_email(
    user: &users::Model,
    password_valid: bool,
    recovery_email: String,
) -> Result<SetRecoveryEmailEffect, AppError> {
    if !password_valid {
        return Err(AppError::Unauthorized("Invalid password".to_string()));
    }

    if recovery_email == user.email {
        return Err(AppError::BadRequest(
            "Recovery email must be different from your primary email".to_string(),
        ));
    }

    if user.recovery_email.as_deref() == Some(&recovery_email) {
        return Err(AppError::BadRequest(
            "This is already your recovery email".to_string(),
        ));
    }

    Ok(SetRecoveryEmailEffect { recovery_email })
}

pub struct SetRecoveryEmailEffect {
    pub recovery_email: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn dummy_user(email: &str, recovery_email: Option<&str>) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            account_status: 1,
            role_id: 1,
            email: email.to_string(),
            full_name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "123".to_string(),
            province: None,
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            recovery_email: recovery_email.map(|s| s.to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_invalid_password() {
        let user = dummy_user("a@b.com", None);
        let result = decide_set_recovery_email(&user, false, "backup@b.com".to_string());
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn test_same_as_primary_email() {
        let user = dummy_user("a@b.com", None);
        let result = decide_set_recovery_email(&user, true, "a@b.com".to_string());
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_same_as_existing_recovery_email() {
        let user = dummy_user("a@b.com", Some("backup@b.com"));
        let result = decide_set_recovery_email(&user, true, "backup@b.com".to_string());
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[test]
    fn test_success() {
        let user = dummy_user("a@b.com", None);
        let result = decide_set_recovery_email(&user, true, "backup@b.com".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().recovery_email, "backup@b.com");
    }
}
