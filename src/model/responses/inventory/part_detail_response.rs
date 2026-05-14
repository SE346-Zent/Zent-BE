use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Detailed view of a single part including audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartDetailResponse {
    pub part_id: Uuid,
    pub part_number: String,
    pub part_type_id: i32,
    pub part_type_name: String,
    pub model_code: Option<String>,
    pub serial_number: String,
    pub description: Option<String>,
    pub condition_id: i32,
    pub condition_name: String,
    pub product_id: Option<Uuid>,
    pub product_name: Option<String>,
    pub manufactured_date: Option<String>,
    pub installation_date: Option<String>,
    /// Approval status: "pending", "approved", "denied"
    pub approval_status: String,
    /// Denial reason (only present when denied)
    pub denial_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
