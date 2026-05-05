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
    #[validate(length(min = 3, max = 15))] // At least 3 images total, up to 15 (5 per phase)
    pub images: Vec<PhaseImage>,
    pub part_changes: Vec<PartChangeInput>,
    #[validate(length(min = 1))]
    pub diagnosis: String,
    #[validate(url)]
    pub signature_url: String,
}
