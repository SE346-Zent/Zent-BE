use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

/// Request payload for initiating a forgotten password flow.
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct ForgotPasswordRequest {
    /// User's email address.
    #[validate(email)]
    pub email: String,
}
