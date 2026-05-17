use crate::{core::errors::AppError, entities::sessions};
use uuid::Uuid;

pub struct LogoutEffect {
    pub session_id: Uuid,
}

/// Pure logic to decide the outcome of a logout attempt.
pub fn decide_logout(session: &sessions::Model, user_id: Uuid) -> Result<LogoutEffect, AppError> {
    // 1. Verify session belongs to user
    if session.user_id != user_id {
        return Err(AppError::Unauthorized(
            "Session does not belong to user".to_string(),
        ));
    }

    // 2. Check if already revoked
    if session.revoked_at.is_some() {
        return Err(AppError::BadRequest("Session already revoked".to_string()));
    }

    Ok(LogoutEffect {
        session_id: session.id,
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
        let session = mock_session(session_user_id, already_revoked);

        let result = decide_logout(&session, user_id);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                assert_eq!(result.unwrap().session_id, session.id);
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
