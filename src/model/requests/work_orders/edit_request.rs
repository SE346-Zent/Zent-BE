use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Request payload for a customer-initiated work order edit.
///
/// All fields are optional — only the ones that are provided will be updated.
/// When `product_id` is changed, the new product must be covered by an active warranty
/// for the customer; otherwise the request will be rejected with a descriptive error.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditWorkOrderRequest {
    /// New product to be serviced. The new product must belong to the requesting
    /// customer and be covered by an active warranty.
    pub product_id: Option<Uuid>,

    /// New reported symptom ID.
    pub work_order_symptom_id: Option<i32>,

    /// New reference ticket ID from an external system.
    pub reference_ticket_id: Option<Uuid>,

    /// New detailed description of the issue.
    #[validate(length(min = 1, max = 1000))]
    pub description: Option<String>,

    /// New service location: Ward.
    #[validate(length(min = 1, max = 255))]
    pub ward: Option<String>,

    /// New service location: Address line 1.
    #[validate(length(min = 1, max = 500))]
    pub address: Option<String>,

    /// New service location: Building/Apartment info.
    #[validate(length(max = 255))]
    pub building: Option<String>,
}
