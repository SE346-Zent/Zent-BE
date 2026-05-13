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
