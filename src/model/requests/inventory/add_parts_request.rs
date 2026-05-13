use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddPartsRequest {
    #[validate(length(min = 1, max = 255))]
    pub part_number: String,
    pub part_types_id: i32,
    #[validate(length(max = 255))]
    pub model_code: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    /// OCI object names for photos (already uploaded or to be uploaded separately)
    #[serde(default)]
    #[validate(length(max = 5))]
    pub photos: Vec<String>,
}
