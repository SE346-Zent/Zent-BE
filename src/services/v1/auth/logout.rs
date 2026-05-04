use crate::{
    core::errors::AppError,
    entities::sessions,
};
use uuid::Uuid;

pub struct LogoutEffect {
    pub session_id: Uuid,
}

/// Pure logic to decide the outcome of a logout attempt.
pub fn decide_logout(
    session: &sessions::Model,
    user_id: Uuid,
) -> Result<LogoutEffect, AppError> {
    // 1. Verify session belongs to user
    if session.user_id != user_id {
        return Err(AppError::Unauthorized("Session does not belong to user".to_string()));
    }

    // 2. Check if already revoked
    if session.revoked_at.is_some() {
        return Err(AppError::BadRequest("Session already revoked".to_string()));
    }

    Ok(LogoutEffect {
        session_id: session.id,
    })
}
