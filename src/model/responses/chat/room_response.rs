use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomResponse {
    pub id: Uuid,
    pub room_name: String,
    pub created_by: Uuid,
    pub created_by_name: String,
    pub member_count: u64,
    pub last_message: Option<LastMessagePreview>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LastMessagePreview {
    pub content: String,
    pub sender_name: String,
    pub sent_at: String,
}
