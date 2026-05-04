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
