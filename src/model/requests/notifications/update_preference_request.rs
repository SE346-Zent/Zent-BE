use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request to toggle OS notification delivery for a single category.
///
/// In-app delivery is always enabled and cannot be changed through this
/// endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationPreferenceRequest {
    /// The category id to update.
    #[validate(range(min = 1))]
    pub category_id: i32,

    /// Whether OS push notifications (FCM) should be delivered for
    /// this category.  `false` silences the OS notification only;
    /// the in-app notification is still created.
    pub os_enabled: bool,
}
