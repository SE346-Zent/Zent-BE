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

/// Full work order history detail: state transitions, optional closing form, optional rating.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderHistoryDetail {
    pub state_history: Vec<WorkOrderStateHistoryEntry>,
    pub closing_form: Option<ClosingFormEntry>,
    pub rating: Option<RatingEntry>,
}
