use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

/// Request payload for resetting a password using a reset token.
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct ResetPasswordRequest {
    /// The token received after OTP verification.
    pub reset_token: String,
    /// The new password (minimum 8 characters).
    #[validate(length(min = 8))]
    pub new_password: String,
}
