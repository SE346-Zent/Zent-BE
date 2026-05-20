use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Represents a part being installed or uninstalled during a service.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartChangeInput {
    /// The unique identifier of the part.
    pub part_id: Uuid,
    /// The type of change: 'installed' or 'uninstalled'.
    pub change_type: String, 
}

/// Represents the result of a checklist item verification.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistResultInput {
    /// The ID of the checklist item.
    pub id: i32,
    /// The verification result (true for pass, false for fail).
    pub result: bool,
    /// Optional notes regarding the verification.
    pub notes: Option<String>,
}

/// Request payload for completing a work order.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkOrderRequest {
    /// Machine Type Model of the serviced product.
    #[validate(length(min = 1))]
    pub mtm: String,
    /// Serial number of the serviced product.
    #[validate(length(min = 1))]
    pub serial_number: String,
    /// List of parts replaced or handled during the service.
    pub part_changes: Vec<PartChangeInput>,
    /// Technician's diagnosis and notes on the service.
    #[validate(length(min = 1))]
    pub diagnosis: String,
    /// Geolocation latitude at completion time.
    pub latitude: f64,
    /// Geolocation longitude at completion time.
    pub longitude: f64,
    /// The filename of the uploaded customer signature image.
    #[validate(length(min = 1))]
    pub signature_file_name: String,
    /// Optional verification checklist results.
    pub checklist: Option<Vec<ChecklistResultInput>>,
}
