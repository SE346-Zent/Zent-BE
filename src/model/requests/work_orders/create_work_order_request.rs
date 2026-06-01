use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Request payload for creating a new work order.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate, Clone)]
pub struct CreateWorkOrderRequest {
    /// The product being serviced.
    pub product_id: Uuid,
    /// The reported symptom ID.
    pub work_order_symptom_id: i32,
    /// Optional reference ticket ID from external systems.
    pub reference_ticket_id: Option<Uuid>,
    /// Detailed description of the issue.
    #[validate(length(min = 1, max = 1000))]
    pub description: String,
    /// Scheduled appointment time.
    pub appointment: DateTime<Utc>,
    /// Customer's first name.
    #[validate(length(min = 1, max = 255))]
    pub first_name: String,
    /// Customer's last name.
    #[validate(length(min = 1, max = 255))]
    pub last_name: String,
    /// Customer's contact email.
    #[validate(email)]
    pub email: Option<String>,
    /// Customer's contact phone number.
    pub phone_number: Option<String>,
    /// Service location: Country.
    pub country: String,
    /// Service location: Province/State.
    pub province: String,
    /// Service location: Ward.
    pub ward: String,
    /// Service location: Address line 1.
    pub address: String,
    /// Service location: Building/Apartment info.
    pub building: Option<String>,
}
