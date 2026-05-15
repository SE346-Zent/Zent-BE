use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Response for a successful product registration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterProductResponse {
    pub product_id: Uuid,
    pub serial_number: String,
    pub message: String,
    /// Whether a confirmation email was sent
    pub email_sent: bool,
}
