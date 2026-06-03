use serde::Deserialize;
use validator::Validate;
use utoipa::ToSchema;

/// Request payload for verifying a recovery email via OTP.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRecoveryEmailRequest {
    /// The 6-digit OTP code sent to the recovery email.
    #[validate(length(equal = 6, message = "OTP code must be exactly 6 digits"))]
    pub otp_code: String,
}
