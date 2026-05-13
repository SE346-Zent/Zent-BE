use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Query parameters for listing in-app notifications.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct NotificationListQuery {
    /// Page number (1-based).  Defaults to 1.
    #[validate(range(min = 1))]
    pub page: Option<u32>,

    /// Number of items per page.  Defaults to 20, max 50.
    #[validate(range(min = 1, max = 50))]
    pub limit: Option<u32>,

    /// Optional category filter.
    pub category_id: Option<i32>,
}
