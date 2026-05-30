use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request payload for registering a device by a customer.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    /// Unique serial number of the product.
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,
    /// Customer's residence: Province (HN or HCM).
    #[validate(length(min = 1, max = 100))]
    pub province: String,
    /// Customer's residence: Ward.
    #[validate(length(min = 1, max = 255))]
    pub ward: String,
    /// Customer's residence: Full address.
    #[validate(length(min = 1, max = 500))]
    pub address: String,
    /// Customer's first name.
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,
    /// Customer's last name.
    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
    /// Customer's contact email.
    #[validate(email)]
    pub email: String,
    /// Customer's contact mobile phone.
    #[validate(length(min = 8, max = 20))]
    pub mobile_phone: String,
    /// Whether to send a confirmation email on successful registration.
    #[serde(default)]
    pub send_email_confirmation: bool,
}
