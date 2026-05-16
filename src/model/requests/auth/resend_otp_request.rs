use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for resending an OTP.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResendOtpRequest {
    /// User's email address.
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}
