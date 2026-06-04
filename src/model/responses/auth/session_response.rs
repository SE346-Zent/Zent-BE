use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: Uuid,
    pub device_name: String,
    pub ip_address: String,
    pub is_current: bool,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
