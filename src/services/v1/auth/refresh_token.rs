use crate::{
    core::{errors::AppError, state::AccessTokenDefaultTTLSeconds},
    entities::{sessions, users},
    model::responses::auth::login_response::{AccountStatusEnum, UserInfo},
    services::v1::core::token_service,
};
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Describes the possible outcomes of a refresh token attempt.
pub enum RefreshTokenEffect {
    /// The token was successfully refreshed, returning new credentials and session info.
    Success {
        /// Basic user information for the response.
        user_info: UserInfo,
        /// The new access and refresh token pair.
        token_bundle: token_service::TokenBundle,
        /// The unique ID of the current session.
        session_id: uuid::Uuid,
        /// Remaining time-to-live for the session in seconds.
        remaining_session_ttl: u64,
    },
    /// A refresh token reuse attack was detected (the provided token is not the active one).
    ReuseAttackDetected {
        /// The unique ID of the compromised session to be revoked.
        session_id: uuid::Uuid,
    },
}

/// Determine the outcome of a token refresh attempt based on session validity and token active state.
///
/// This pure function validates that the session is still active and, crucially,
/// performs a detection for token reuse attacks by comparing the provided token's
/// hash with the currently active hash from the whitelist cache.
///
/// # Arguments
/// * `session_record` - The database model representing the user's current session.
/// * `user_record` - The database model of the user owning the session.
/// * `active_refresh_token_hash` - The currently whitelisted token hash for this session (from Valkey).
/// * `provided_refresh_token_hash` - The hash of the token provided in the refresh request.
/// * `access_token_ttl` - Duration for which the new access token will be valid.
/// * `encoding_key` - Key used to sign the new tokens.
///
/// # Returns
/// A result containing the `RefreshTokenEffect` (Success or ReuseAttackDetected), or an `AppError`.
pub fn decide_refresh_token(
    session_record: &sessions::Model,
    user_record: &users::Model,
    active_refresh_token_hash: Option<String>,
    provided_refresh_token_hash: &str,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<RefreshTokenEffect, AppError> {
    if session_record.revoked_at.is_some() || session_record.expires_at < Utc::now() {
        tracing::warn!(
            reason = "SessionInvalidOrExpired",
            session_id = %session_record.id,
            user_id = %user_record.id,
            "Token refresh failed: session is revoked or expired"
        );
        return Err(AppError::Unauthorized(
            "Session invalid or expired".to_string(),
        ));
    }

    if active_refresh_token_hash.as_deref() != Some(provided_refresh_token_hash) {
        tracing::error!(
            reason = "RefreshTokenReuseAttack",
            session_id = %session_record.id,
            user_id = %user_record.id,
            "Token refresh failed: refresh token reuse attack detected"
        );
        return Ok(RefreshTokenEffect::ReuseAttackDetected {
            session_id: session_record.id,
        });
    }

    let token_bundle = token_service::generate_token_bundle(
        &user_record.id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    let remaining_duration_seconds = (session_record.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;

    tracing::info!(
        session_id = %session_record.id,
        user_id = %user_record.id,
        "Token refreshed successfully"
    );

    Ok(RefreshTokenEffect::Success {
        user_info: UserInfo {
            full_name: user_record.full_name.clone(),
            account_status: AccountStatusEnum::from(user_record.account_status),
            email: user_record.email.clone(),
            phone_number: user_record.phone_number.clone(),
            role_id: user_record.role_id,
        },
        token_bundle,
        session_id: session_record.id,
        remaining_session_ttl: remaining_duration_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{sessions, users};
    use chrono::Utc;
    use jsonwebtoken::EncodingKey;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_key() -> EncodingKey {
        EncodingKey::from_secret(b"secret")
    }

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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[fixture]
    fn mock_session(
        #[default(Uuid::new_v4())] user_id: Uuid,
        #[default(3600)] expires_in_sec: i64,
        #[default(false)] revoked: bool,
    ) -> sessions::Model {
        sessions::Model {
            id: Uuid::new_v4(),
            user_id,
            refresh_token_hash: "valid_hash".to_string(),
            ip_address: "127.0.0.1".to_string(),
            device_fingerprint: "fp".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in_sec),
            revoked_at: if revoked { Some(Utc::now()) } else { None },
        }
    }

    #[rstest]
    // Success cases
    #[case(false, false, Some(true), "Success")]
    // Failure cases: Unauthorized (Revoked/Expired)
    #[case(true, true, Some(true), "Unauthorized")]
    #[case(true, true, Some(false), "Unauthorized")]
    #[case(true, false, Some(true), "Unauthorized")]
    #[case(true, false, Some(false), "Unauthorized")]
    #[case(false, true, Some(true), "Unauthorized")]
    #[case(false, true, Some(false), "Unauthorized")]
    // Failure case: Reuse attack
    #[case(false, false, Some(false), "ReuseAttack")]
    #[case(false, false, None, "ReuseAttack")]
    /// Control whether active_refresh_token_hash input to decide function will match fixed current_hash value
    fn test_decide_refresh_token_exhaustive(
        #[case] revoked: bool,
        #[case] expired: bool,
        #[case] hash_matches: Option<bool>,
        #[case] expected: &str,
        mock_user: users::Model,
        mock_key: EncodingKey,
    ) {
        let expires_in = if expired { -10 } else { 3600 };
        let session = mock_session(mock_user.id, expires_in, revoked);
        let active_refresh_token_hash = match hash_matches {
            Some(true) => Some("valid_hash".to_string()),
            Some(false) => Some("different_hash".to_string()),
            None => None,
        };

        let result = decide_refresh_token(
            &session,
            &mock_user,
            active_refresh_token_hash,
            "valid_hash",
            AccessTokenDefaultTTLSeconds(900),
            &mock_key,
        );

        match expected {
            "Success" => {
                assert!(result.is_ok());
                if let RefreshTokenEffect::Success { user_info, .. } = result.unwrap() {
                    assert_eq!(user_info.email, mock_user.email);
                } else {
                    panic!("Expected Success effect");
                }
            }
            "ReuseAttack" => {
                assert!(result.is_ok());
                if let RefreshTokenEffect::ReuseAttackDetected { session_id } = result.unwrap() {
                    assert_eq!(session_id, session.id);
                } else {
                    panic!("Expected ReuseAttackDetected effect");
                }
            }
            "Unauthorized" => {
                assert!(matches!(result, Err(AppError::Unauthorized(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
