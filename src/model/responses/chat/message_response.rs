use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageResponse {
    pub id: String,
    pub room_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: Option<String>,
    pub image_url: Option<String>,
    pub reply_to: Option<String>,
    pub created_at: String,
    pub edited_at: Option<String>,
    pub read_by: Vec<String>,
}
