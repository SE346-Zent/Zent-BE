use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single user notification-preference entry.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPreferenceResponse {
    pub category_id: i32,
    pub category_name: String,
    pub category_slug: String,

    /// Whether OS push (FCM) delivery is enabled.
    /// `true` by default for every category.
    pub os_enabled: bool,

    /// When the preference was last modified.
    pub updated_at: Option<String>,
}
