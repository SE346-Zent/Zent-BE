use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChatRoomResponse {
    pub id: Uuid,
    /// Display name of the opposite user in this 1-on-1 chat.
    pub opposite_user_name: String,
    /// Avatar URL of the opposite user.
    pub opposite_avatar_url: Option<String>,
    /// Preview of the most recent message (content snippet).
    pub latest_message: Option<String>,
    /// Timestamp of the most recent message (ISO-8601 string).
    pub latest_message_at: Option<String>,
    /// Count of unread messages for the current user.
    pub unread_count: u64,
}
