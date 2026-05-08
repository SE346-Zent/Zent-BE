use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Full detail for a single in-app notification.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDetailResponse {
    pub notification_id: String,
    pub category_id: i32,
    pub category_name: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,

    /// The 1:1 linked OS notification id, if one was sent (i.e. the
    /// user had OS delivery enabled at the time).
    pub os_notification_id: Option<String>,

    pub created_at: String,
}
