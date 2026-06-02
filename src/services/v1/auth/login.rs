use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::users,
    model::responses::auth::login_response::{AccountStatusEnum, LoginResponseData, UserInfo},
    services::v1::core::token_service,
};

use sea_orm::Set;
use crate::entities::sessions;
use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Represents the calculated results and side-effects of a successful login attempt.
///
/// This structure decouples the pure business logic of deciding a login outcome from the
/// infrastructure-heavy tasks of persisting sessions and responding to the HTTP request.
pub struct LoginEffect {
    /// Unique identifier for the newly created session.
    pub session_id: Uuid,
    /// The unique identifier of the user who logged in.
    pub user_id: Uuid,
    /// A cryptographic hash of the refresh token for secure server-side storage.
    pub refresh_token_hash: String,
    /// The timestamp when this session and its refresh token will expire.
    pub session_expires_at: chrono::DateTime<Utc>,
    /// The final data structure to be returned to the client in the API response.
    pub response_data: LoginResponseData,
}

/// Determine the outcome of a login attempt based on user state and credentials.
///
/// This is a pure function that validates the user's account status and generates
/// authentication tokens, returning a `LoginEffect` describing the necessary side-effects.
///
/// # Arguments
/// * `user_record` - The database model representing the user attempting to log in.
/// * `is_password_valid` - Boolean indicating if the provided password matches the stored hash.
/// * `access_token_ttl` - The duration for which the access token should remain valid.
/// * `session_ttl` - The duration for which the user session and refresh token should remain valid.
/// * `encoding_key` - The cryptographic key used to sign the generated JWTs.
///
/// # Returns
/// A result containing the `LoginEffect` on success, or an `AppError` (e.g., `Unauthorized`, `Forbidden`).
pub fn decide_login(
    user_record: &users::Model,
    is_password_valid: bool,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<LoginEffect, AppError> {
    // 1. Check if user is deleted
    if user_record.deleted_at.is_some() {
        tracing::warn!(
            error.message = "AccountDeactivated", error.details = "",
            user_id = %user_record.id,
            email = %user_record.email,
            "Login failed: account is deactivated/deleted"
        );
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 2. Verify password (passed in)
    if !is_password_valid {
        tracing::warn!(
            error.message = "InvalidPassword", error.details = "",
            user_id = %user_record.id,
            email = %user_record.email,
            "Login failed: invalid password"
        );
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 3. Verify account status
    let account_status = AccountStatusEnum::from(user_record.account_status);
    match account_status {
        AccountStatusEnum::Active => {}
        AccountStatusEnum::Pending => {
            tracing::warn!(
                error.message = "AccountPending", error.details = "",
                user_id = %user_record.id,
                email = %user_record.email,
                "Login failed: account is pending verification"
            );
            return Err(AppError::Forbidden(
                "Account is pending verification".to_string(),
            ));
        }
        _ => {
            tracing::warn!(
                error.message = "AccountNotActive", error.details = "",
                user_id = %user_record.id,
                email = %user_record.email,
                status = ?account_status,
                "Login failed: account status is not active"
            );
            return Err(AppError::Forbidden(format!("Account is {:?}", account_status)));
        }
    }

    // 4. Generate tokens
    let token_bundle = token_service::generate_token_bundle(
        &user_record.id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    // 5. Prepare session ActiveModel
    let session_id = Uuid::new_v4();
    let session_duration_seconds = session_ttl.0;
    let session_expires_at = Utc::now() + chrono::Duration::seconds(session_duration_seconds);

    tracing::info!(
        user_id = %user_record.id,
        email = %user_record.email,
        "Login succeeded"
    );

    Ok(LoginEffect {
        session_id,
        user_id: user_record.id,
        refresh_token_hash: token_bundle.refresh_token_hash,
        session_expires_at,
        response_data: LoginResponseData {
            user: UserInfo {
                full_name: user_record.full_name.clone(),
                account_status,
                email: user_record.email.clone(),
                phone_number: user_record.phone_number.clone(),
                role_id: user_record.role_id,
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
            avatar_url: None,
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

        let duration = (effect.session_expires_at - before_call).num_seconds();

        // Allow a 1-2 second buffer for execution time depending on system speed
        assert!(duration >= expected_duration && duration <= expected_duration + 2);
    }
}
