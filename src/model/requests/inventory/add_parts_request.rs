use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request payload for adding new parts to the inventory.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddPartsRequest {
    /// The manufacturer's part number.
    #[validate(length(min = 1, max = 255))]
    pub part_number: String,
    /// The ID of the part type.
    pub part_types_id: i32,
    /// Optional model code if specific to a model.
    #[validate(length(max = 255))]
    pub model_code: Option<String>,
    /// Unique serial number of the part.
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,
    /// Optional description of the part condition or details.
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    /// The associated work order number.
    #[validate(length(max = 255))]
    pub work_order_number: String,
    /// OCI object names for photos (already uploaded or to be uploaded separately).
    #[serde(default)]
    #[validate(length(max = 5))]
    pub photos: Vec<String>,
}
