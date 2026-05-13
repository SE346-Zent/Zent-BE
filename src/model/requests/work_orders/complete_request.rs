use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartChangeInput {
    pub part_id: Uuid,
    pub change_type: String, // 'installed', 'uninstalled'
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistResultInput {
    pub id: i32,
    pub result: bool,
    pub notes: Option<String>,
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
    pub signature_file_name: String,
    pub checklist: Option<Vec<ChecklistResultInput>>,
}
