use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for OTP verification.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyOtpRequest {
    /// User's email address.
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    /// The 6-digit OTP code.
    #[validate(length(equal = 6, message = "OTP must be exactly 6 digits"))]
    pub otp_code: String,
}
