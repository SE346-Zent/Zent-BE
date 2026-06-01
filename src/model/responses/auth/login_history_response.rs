use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginHistoryEntry {
    pub id: Uuid,
    pub session_id: Uuid,
    pub device_name: String,
    pub location: Option<String>,
    pub ip_address: String,
    pub created_at: DateTime<Utc>,
}