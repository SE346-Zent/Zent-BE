use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Summary view of a part for list queries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartListItem {
    pub part_id: Uuid,
    pub part_number: String,
    pub part_type_name: String,
    pub serial_number: String,
    pub condition_name: String,
    /// Name of the product this part is installed in (if any)
    pub product_name: Option<String>,
    /// Approval status: "pending", "approved", or "rejected"
    pub approval_status: String,
    pub created_at: String,
}
