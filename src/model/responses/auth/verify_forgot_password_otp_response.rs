use serde::{Deserialize, Serialize};

/// Data returned after successful forgot password OTP verification
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyForgotPasswordOtpResponseData {
    /// The token used to authorize the final password reset step
    pub reset_token: String,
}
