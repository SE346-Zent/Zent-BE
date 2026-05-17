use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    /// Chat room ID
    pub room_id: String,
    /// Optional text content
    #[validate(length(min = 0, max = 5000))]
    pub content: Option<String>,
    /// Optional image URL (uploaded beforehand via attachment endpoint)
    pub image_url: Option<String>,
    /// Optional message ID this message is replying to
    pub reply_to: Option<String>,
}
