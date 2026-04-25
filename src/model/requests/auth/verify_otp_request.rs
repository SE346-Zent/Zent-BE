use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyOtpRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    
    #[validate(length(equal = 6, message = "OTP must be exactly 6 digits"))]
    pub otp_code: String,
}
