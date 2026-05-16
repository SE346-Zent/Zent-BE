use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single user notification-preference entry.
/// Response containing a user's notification preference for a specific category.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferenceResponse {
    /// ID of the notification category.
    pub category_id: i32,
    /// Human-readable name of the category.
    pub category_name: String,
    /// URL-friendly identifier for the category.
    pub category_slug: String,

    /// Whether OS-level push notifications (FCM) are enabled for this category.
    pub os_enabled: bool,

    /// Timestamp when this preference was last modified.
    pub updated_at: Option<String>,
}
