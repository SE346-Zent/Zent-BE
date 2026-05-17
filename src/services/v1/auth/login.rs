use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::users,
    model::responses::auth::login_response::{AccountStatusEnum, LoginResponseData, UserInfo},
    services::v1::core::token_service,
};

use chrono::Utc;
use jsonwebtoken::EncodingKey;
use uuid::Uuid;

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
            return Err(AppError::Forbidden(
                "Account is pending verification".to_string(),
            ));
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
    let expires_at = Utc::now() + chrono::Duration::seconds(session_ttl_seconds);

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
    use crate::model::responses::auth::login_response::AccountStatusEnum;
    use chrono::Utc;
    use jsonwebtoken::EncodingKey;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_key() -> EncodingKey {
        EncodingKey::from_secret(b"secret")
    }

    #[fixture]
    fn mock_user(
        #[default(AccountStatusEnum::Active)] status: AccountStatusEnum,
        #[default(false)] deleted: bool,
    ) -> users::Model {
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

    #[rstest]
    fn test_decide_login_success(mock_user: users::Model, mock_key: EncodingKey) {
        let result = decide_login(
            &mock_user,
            true, // valid password
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        );

        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.user_id, mock_user.id);
        assert_eq!(effect.response_data.user.email, mock_user.email);
    }

    #[rstest]
    // If user is deleted, it's always Unauthorized
    #[case(AccountStatusEnum::Active, true, true, "Unauthorized")]
    #[case(AccountStatusEnum::Pending, true, true, "Unauthorized")]
    #[case(AccountStatusEnum::Locked, true, true, "Unauthorized")]
    #[case(AccountStatusEnum::Inactive, true, true, "Unauthorized")]
    #[case(AccountStatusEnum::Terminated, true, true, "Unauthorized")]
    #[case(AccountStatusEnum::from(10), true, true, "Unauthorized")]
    // If user is deleted AND password invalid, it's Unauthorized (checks deleted first)
    #[case(AccountStatusEnum::Active, false, true, "Unauthorized")]
    #[case(AccountStatusEnum::Pending, false, true, "Unauthorized")]
    #[case(AccountStatusEnum::Locked, false, true, "Unauthorized")]
    #[case(AccountStatusEnum::Inactive, false, true, "Unauthorized")]
    #[case(AccountStatusEnum::Terminated, false, true, "Unauthorized")]
    #[case(AccountStatusEnum::from(12), false, true, "Unauthorized")]
    // If user is NOT deleted but password invalid, it's Unauthorized
    #[case(AccountStatusEnum::Active, false, false, "Unauthorized")]
    #[case(AccountStatusEnum::Pending, false, false, "Unauthorized")]
    #[case(AccountStatusEnum::Locked, false, false, "Unauthorized")]
    #[case(AccountStatusEnum::Inactive, false, false, "Unauthorized")]
    #[case(AccountStatusEnum::Terminated, false, false, "Unauthorized")]
    #[case(AccountStatusEnum::from(13), false, false, "Unauthorized")]
    // If user is NOT deleted, password IS valid, but account status is not Active, it's Forbidden
    #[case(AccountStatusEnum::Pending, true, false, "Forbidden")]
    #[case(AccountStatusEnum::Locked, true, false, "Forbidden")]
    #[case(AccountStatusEnum::Inactive, true, false, "Forbidden")]
    #[case(AccountStatusEnum::Terminated, true, false, "Forbidden")]
    #[case(AccountStatusEnum::from(11), true, false, "Forbidden")]
    fn test_decide_login_invalid_cases(
        #[case] account_status: AccountStatusEnum,
        #[case] is_password_valid: bool,
        #[case] is_user_deleted: bool,
        #[case] expected_error: &str,
        mock_key: EncodingKey,
    ) {
        let mock_user = mock_user(account_status, is_user_deleted);
        let result = decide_login(
            &mock_user,
            is_password_valid,
            AccessTokenDefaultTTLSeconds(900),
            SessionDefaultTTLSeconds(3600),
            &mock_key,
        );

        match expected_error {
            "Unauthorized" => assert!(matches!(result, Err(AppError::Unauthorized(_)))),
            "Forbidden" => assert!(matches!(result, Err(AppError::Forbidden(_)))),
            _ => panic!("Unknown expected error type"),
        }
    }

    #[rstest]
    #[case(0, 0, 0)] // Zero TTL (Immediate expiration)
    #[case(1, 1, 1)] // 1 Second TTL
    #[case(31536000, 31536000, 31536000)] // 1 Year TTL
    #[case(900, 3153600000, 3153600000)] // 100 Years TTL
    fn test_decide_login_ttl_boundaries(
        #[case] access_ttl: i64,
        #[case] session_ttl: i64,
        #[case] expected_duration: i64,
        mock_user: users::Model,
        mock_key: EncodingKey,
    ) {
        let before_call = Utc::now();
        let result = decide_login(
            &mock_user,
            true,
            AccessTokenDefaultTTLSeconds(access_ttl),
            SessionDefaultTTLSeconds(session_ttl),
            &mock_key,
        );

        assert!(result.is_ok());
        let effect = result.unwrap();

        let duration = (effect.expires_at - before_call).num_seconds();

        // Allow a 1-2 second buffer for execution time depending on system speed
        assert!(duration >= expected_duration && duration <= expected_duration + 2);
    }
}
