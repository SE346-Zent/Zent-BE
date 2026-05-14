use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProductRequest {
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,
    /// Auto-assigned to "Vietnam" by default
    #[validate(length(min = 1, max = 100))]
    pub country: String,
    #[validate(length(min = 1, max = 100))]
    pub province: String,
    #[validate(length(min = 1, max = 100))]
    pub city: String,
    #[validate(length(min = 1, max = 500))]
    pub address: String,
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,
    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 20))]
    pub mobile_phone: String,
    /// Whether to send a confirmation email on successful registration
    #[serde(default)]
    pub send_email_confirmation: bool,
}
