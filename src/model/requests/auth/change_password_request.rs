use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for changing the authenticated user's password.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    /// The user's current password.
    #[validate(length(min = 6, message = "Old password must be at least 6 characters"))]
    pub old_password: String,

    /// The new password to set.
    #[validate(length(min = 6, message = "New password must be at least 6 characters"))]
    pub new_password: String,
}
