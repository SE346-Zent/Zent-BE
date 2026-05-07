use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhaseImage {
    #[validate(length(min = 1))]
    pub phase: String, // 'pre-disassembly', 'disassembled', 'post-assembly'
    #[validate(url)]
    pub image_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartChangeInput {
    pub part_id: Uuid,
    pub change_type: String, // 'installed', 'uninstalled'
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompleteWorkOrderRequest {
    #[validate(length(min = 1))]
    pub mtm: String,
    #[validate(length(min = 1))]
    pub serial_number: String,
    pub part_changes: Vec<PartChangeInput>,
    #[validate(length(min = 1))]
    pub diagnosis: String,
    pub latitude: f64,
    pub longitude: f64,
    #[validate(length(min = 1))]
    pub signature_url: String,
}

// Remove CompleteWorkOrderMultipart as it's no longer needed for detatched flow
// (or leave it if you prefer but we'll use JSON for the main call now)
