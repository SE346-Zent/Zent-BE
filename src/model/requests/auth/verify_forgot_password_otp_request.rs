use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct VerifyForgotPasswordOtpRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(equal = 6))]
    pub otp_code: String,
}
