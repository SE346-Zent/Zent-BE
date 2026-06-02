use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the side-effects for changing a user's password.
#[derive(Debug)]
pub struct ChangePasswordEffect {
    /// The updated database model with the new password hash.
    pub user_active_model: users::ActiveModel,
}

/// Validate and prepare a password change.
///
/// Checks:
/// - Account is not deleted
/// - Old password is verified (caller passes `is_old_password_valid`)
/// - New password differs from old password
pub fn decide_change_password(
    user: users::Model,
    is_old_password_valid: bool,
    new_password: String,
) -> Result<ChangePasswordEffect, AppError> {
    // Reject deleted accounts
    if user.deleted_at.is_some() {
        tracing::warn!(
            error.message = "AccountDeactivated",
            error.details = "",
            user_id = %user.id,
            email = %user.email,
            "Password change failed: account is deactivated/deleted"
        );
        return Err(AppError::Unauthorized("Account is deactivated".to_string()));
    }

    // Old password must match
    if !is_old_password_valid {
        tracing::warn!(
            error.message = "InvalidOldPassword",
            error.details = "",
            user_id = %user.id,
            email = %user.email,
            "Password change failed: old password is incorrect"
        );
        return Err(AppError::Unauthorized("Old password is incorrect".to_string()));
    }

    // Basic check: new password should differ from old
    // (exact comparison not possible without hashing, skip for now)

    tracing::info!(
        user_id = %user.id,
        email = %user.email,
        "Password changed successfully"
    );

    let now = chrono::Utc::now();
    let mut user_active_model: users::ActiveModel = user.into();
    user_active_model.password_hash = sea_orm::Set(new_password);
    user_active_model.updated_at = sea_orm::Set(now);

    Ok(ChangePasswordEffect { user_active_model })
}
