use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Response for a successful device registration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceResponse {
    pub registration_id: Uuid,
    pub product_id: Uuid,
    pub serial_number: String,
    pub message: String,
    /// Whether a confirmation email was sent
    pub email_sent: bool,
    /// Warranty status of the registered device
    pub warranty_status: String,
}
