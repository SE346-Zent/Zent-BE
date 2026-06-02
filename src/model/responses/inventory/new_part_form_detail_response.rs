use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPartFormDetailResponse {
    pub id: Uuid,
    pub part_number: String,
    pub part_type_name: String,
    pub model_code: Option<String>,
    pub serial_number: String,
    pub work_order_id: Uuid,
    pub work_order_number: String,
    pub description: Option<String>,
    pub status: String,
    /// Present when status is approved or rejected.
    pub approver_name: Option<String>,
    /// Present when status is approved.
    pub approved_at: Option<String>,
    /// Present when status is rejected.
    pub rejected_at: Option<String>,
    /// Present when status is rejected.
    pub denial_reason: Option<String>,
    pub photo_urls: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}