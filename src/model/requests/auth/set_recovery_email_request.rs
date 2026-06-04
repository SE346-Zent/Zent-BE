use serde::Deserialize;
use validator::Validate;
use utoipa::ToSchema;

/// Request payload for setting or updating a recovery email address.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetRecoveryEmailRequest {
    /// The recovery email address to register.
    #[validate(email)]
    pub recovery_email: String,

    /// The user's current password for verification.
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,
}
