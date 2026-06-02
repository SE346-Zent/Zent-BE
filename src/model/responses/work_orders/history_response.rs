use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderStateHistoryEntry {
    pub id: Uuid,
    pub changed_by: String,
    pub from_status: Option<String>,
    pub to_status: String,
    pub changed_at: DateTime<Utc>,
}

/// Closing form associated with a completed work order.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClosingFormEntry {
    pub id: Uuid,
    pub mtm: String,
    pub serial_number: String,
    pub diagnosis: String,
    pub signature_file_name: String,
    pub created_at: DateTime<Utc>,
}

/// Rating associated with a work order.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RatingEntry {
    pub rating: i32,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A single part change (installed or uninstalled) during a work order.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PartChangeEntry {
    pub part_id: Uuid,
    pub part_number: String,
    pub serial_number: String,
    pub change_type: String,
    pub created_at: String,
}

/// Full work order history detail: state transitions, optional closing form, optional rating,
/// optional part changes, optional evidence photos.
///
/// For Admin/SuperAdmin: all fields populated.
/// For Technician: state_history is absent; other fields populated.
/// For Customer: only technician_name, status, and ended_at are populated.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderHistoryDetail {
    /// State transition history (absent for Technician and Customer roles).
    pub state_history: Option<Vec<WorkOrderStateHistoryEntry>>,
    pub closing_form: Option<ClosingFormEntry>,
    pub rating: Option<RatingEntry>,
    /// Part changes recorded in the closing form (Admin/SuperAdmin/Technician).
    pub part_changes: Option<Vec<PartChangeEntry>>,
    /// Evidence photo object names from the work order (Admin/SuperAdmin/Technician).
    pub evidence_photos: Option<Vec<String>>,
    // --- Customer-specific fields ---
    /// Assigned technician's full name (Customer only).
    pub technician_name: Option<String>,
    /// Current work order status name (Customer only).
    pub status: Option<String>,
    /// Time the work order ended (Customer only, present when completed).
    pub ended_at: Option<String>,
}
