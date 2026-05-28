use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Detailed information about a work order.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderDetails {
    /// Unique identifier for the work order.
    pub id: Uuid,
    /// Human-readable work order number.
    pub work_order_number: String,
    /// The technician assigned to this work order.
    pub technician_id: Option<Uuid>,
    /// Current status (e.g., 'New', 'Assigned', 'In Progress', 'Completed').
    pub status: String,
    /// The customer who requested the service.
    pub customer_id: Uuid,
    /// Customer's full name.
    pub customer_name: String,
    /// The product being serviced.
    pub product_id: Uuid,
    /// The product's model name.
    pub product_name: String,
    /// Optional reference ticket ID from external systems.
    pub reference_ticket_id: Option<Uuid>,
    /// The name of the reported symptom.
    pub symptom_name: String,
    /// Detailed description of the issue.
    pub description: String,
    /// Customer's first name.
    pub first_name: String,
    /// Customer's last name.
    pub last_name: String,
    /// Customer's contact email.
    pub email: Option<String>,
    /// Customer's contact phone number.
    pub phone_number: Option<String>,
    /// Service location: Country.
    pub country: String,
    /// Service location: Province/State.
    pub province: String,
    /// Service location: City.
    pub city: String,
    /// Service location: Address line 1.
    pub address: String,
    /// Service location: Building/Apartment info.
    pub building: Option<String>,
    /// Scheduled appointment time (GMT+7).
    pub appointment: String,
    /// Timestamp when the work order was created (GMT+7).
    pub created_at: String,
    /// Timestamp when the work order was last updated (GMT+7).
    pub updated_at: String,
}
