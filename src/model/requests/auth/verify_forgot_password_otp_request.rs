use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

/// Request payload for verifying an OTP in the forgotten password flow.
#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct VerifyForgotPasswordOtpRequest {
    /// User's email address.
    #[validate(email)]
    pub email: String,
    /// The 6-digit OTP code.
    #[validate(length(equal = 6))]
    pub otp_code: String,
}
