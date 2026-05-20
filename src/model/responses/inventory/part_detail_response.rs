use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Detailed view of a single part including audit trail.
/// Detailed information about a single part in the inventory.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartDetailResponse {
    /// Unique identifier for the part.
    pub part_id: Uuid,
    /// Manufacturer's part number.
    pub part_number: String,
    /// ID of the part type.
    pub part_type_id: i32,
    /// Human-readable name of the part type.
    pub part_type_name: String,
    /// Associated product model code.
    pub model_code: Option<String>,
    /// Unique serial number of the part.
    pub serial_number: String,
    /// Optional description of the part.
    pub description: Option<String>,
    /// ID of the part's current condition.
    pub condition_id: i32,
    /// Human-readable name of the part's current condition.
    pub condition_name: String,
    /// ID of the product this part is currently installed in.
    pub product_id: Option<Uuid>,
    /// Name of the product this part is currently installed in.
    pub product_name: Option<String>,
    /// Date when the part was manufactured.
    pub manufactured_date: Option<String>,
    /// Date when the part was installed.
    pub installation_date: Option<String>,
    /// Current approval status: 'pending', 'approved', or 'denied'.
    pub approval_status: String,
    /// Reason for denial, if the status is 'denied'.
    pub denial_reason: Option<String>,
    /// Timestamp when the part was registered.
    pub created_at: String,
    /// Timestamp when the part record was last updated.
    pub updated_at: String,
}
