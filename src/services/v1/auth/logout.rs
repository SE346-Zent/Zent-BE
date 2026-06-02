use crate::{core::errors::AppError, entities::sessions};
use uuid::Uuid;

/// Represents the calculated results and side-effects of a successful logout request.
pub struct LogoutEffect {
    /// The unique identifier of the session to be invalidated.
    pub revoked_session_id: Uuid,
}

/// Determine the outcome of a logout attempt by validating session ownership and state.
///
/// This pure function verifies that the session belongs to the requesting user
/// and has not already been revoked.
///
/// # Arguments
/// * `session_record` - The database model representing the session to be revoked.
/// * `requesting_user_id` - The unique identifier of the authenticated user requesting logout.
///
/// # Returns
/// A result containing the `LogoutEffect` on success, or an `AppError` (e.g., `Unauthorized`, `BadRequest`).
pub fn decide_logout(
    session_record: &sessions::Model,
    requesting_user_id: Uuid,
) -> Result<LogoutEffect, AppError> {
    // 1. Verify session belongs to user
    if session_record.user_id != requesting_user_id {
        tracing::warn!(
            error.message = "SessionOwnershipMismatch", error.details = "",
            session_id = %session_record.id,
            session_owner_id = %session_record.user_id,
            requesting_user_id = %requesting_user_id,
            "Logout failed: session does not belong to user"
        );
        return Err(AppError::Unauthorized(
            "Session does not belong to user".to_string(),
        ));
    }

    // 2. Check if already revoked
    if session_record.revoked_at.is_some() {
        tracing::warn!(
            error.message = "SessionAlreadyRevoked", error.details = "",
            session_id = %session_record.id,
            user_id = %requesting_user_id,
            "Logout failed: session is already revoked"
        );
        return Err(AppError::BadRequest("Session already revoked".to_string()));
    }

    tracing::info!(
        session_id = %session_record.id,
        user_id = %requesting_user_id,
        "Logout succeeded"
    );

    Ok(LogoutEffect {
        revoked_session_id: session_record.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::sessions;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn user_id() -> Uuid {
        Uuid::new_v4()
    }

    #[fixture]
    fn mock_session(
        #[default(Uuid::new_v4())] session_user_id: Uuid,
        #[default(false)] revoked: bool,
    ) -> sessions::Model {
        sessions::Model {
            id: Uuid::new_v4(),
            user_id: session_user_id,
            refresh_token_hash: "hash".to_string(),
            ip_address: "127.0.0.1".to_string(),
            device_fingerprint: "fingerprint".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            revoked_at: if revoked { Some(Utc::now()) } else { None },
        }
    }

    #[rstest]
    // Success case
    #[case(true, false, "Ok")]
    // Error cases
    #[case(false, false, "Unauthorized")] // Wrong user
    #[case(true, true, "BadRequest")] // Already revoked
    #[case(false, true, "Unauthorized")] // Wrong user + revoked (Ownership check should fail first)
    fn test_decide_logout_exhaustive(
        #[case] same_user: bool,
        #[case] already_revoked: bool,
        #[case] expected_result: &str,
        user_id: Uuid,
    ) {
        let session_user_id = if same_user { user_id } else { Uuid::new_v4() };
        let session_record = mock_session(session_user_id, already_revoked);

        let result = decide_logout(&session_record, user_id);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                assert_eq!(result.unwrap().revoked_session_id, session_record.id);
            }
            "Unauthorized" => {
                assert!(matches!(result, Err(AppError::Unauthorized(_))));
            }
            "BadRequest" => {
                assert!(matches!(result, Err(AppError::BadRequest(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
