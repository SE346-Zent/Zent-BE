use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::users,
    model::responses::auth::login_response::{LoginResponseData, UserInfo, AccountStatusEnum},
    services::v1::core::token_service,
};

use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Plain struct representing the side-effects that need to be persisted
pub struct LoginEffect {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub response_data: LoginResponseData,
}

/// Pure logic to decide the outcome of a login attempt.
pub fn decide_login(
    user_model: &users::Model,
    is_password_valid: bool,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<LoginEffect, AppError> {
    // 1. Check if user is deleted
    if user_model.deleted_at.is_some() {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 2. Verify password (passed in)
    if !is_password_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 3. Verify account status
    let status = AccountStatusEnum::from(user_model.account_status);
    match status {
        AccountStatusEnum::Active => {} 
        AccountStatusEnum::Pending => {
            return Err(AppError::Forbidden("Account is pending verification".to_string()));
        }
        _ => {
            return Err(AppError::Forbidden(format!("Account is {:?}", status)));
        }
    }

    // 4. Generate tokens
    let token_bundle = token_service::generate_token_bundle(
        &user_model.id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    // 5. Prepare session data
    let session_id = Uuid::new_v4();
    let session_ttl_seconds = session_ttl.0;
    let expires_at = Utc::now() + chrono::Duration::seconds(session_ttl_seconds as i64);

    Ok(LoginEffect {
        session_id,
        user_id: user_model.id,
        refresh_token_hash: token_bundle.refresh_token_hash,
        expires_at,
        response_data: LoginResponseData {
            user: UserInfo {
                full_name: user_model.full_name.clone(),
                account_status: status,
                email: user_model.email.clone(),
                phone_number: user_model.phone_number.clone(),
                role_id: user_model.role_id,
            },
            access_token: token_bundle.access_token,
            refresh_token: token_bundle.refresh_token,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use jsonwebtoken::EncodingKey;
    use crate::model::responses::auth::login_response::AccountStatusEnum;

    fn get_mock_key() -> EncodingKey {
        EncodingKey::from_secret(b"secret")
    }

    fn create_mock_user(status: AccountStatusEnum, deleted: bool) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: i32::from(status),
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: if deleted { Some(Utc::now()) } else { None },
        }
    }

    #[test]
    fn test_decide_login_success() {
        let user = create_mock_user(AccountStatusEnum::Active, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true, // valid password
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.user_id, user.id);
        assert_eq!(effect.response_data.user.email, user.email);
    }

    #[test]
    fn test_decide_login_invalid_password() {
        let user = create_mock_user(AccountStatusEnum::Active, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            false, // invalid password
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn test_decide_login_deleted_user() {
        let user = create_mock_user(AccountStatusEnum::Active, true); // logically deleted
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn test_decide_login_status_pending() {
        let user = create_mock_user(AccountStatusEnum::Pending, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_decide_login_status_inactive() {
        let user = create_mock_user(AccountStatusEnum::Inactive, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_decide_login_status_locked() {
        let user = create_mock_user(AccountStatusEnum::Locked, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_decide_login_status_terminated() {
        let user = create_mock_user(AccountStatusEnum::Terminated, false);
        let key = get_mock_key();
        
        let result = decide_login(
            &user,
            true,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &key,
        );

        assert!(matches!(result, Err(AppError::Forbidden(_))));
    }
}
