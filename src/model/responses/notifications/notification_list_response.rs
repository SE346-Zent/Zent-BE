use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single in-app notification item returned in list views.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListItem {
    pub notification_id: String,
    pub category_id: i32,
    pub category_name: String,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: String,
}
