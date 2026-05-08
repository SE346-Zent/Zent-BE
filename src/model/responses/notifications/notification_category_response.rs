use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A notification category definition.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NotificationCategoryResponse {
    pub id: i32,
    pub name: String,
    /// URL-safe identifier, e.g. "work_order_assigned".
    pub slug: String,
    /// Human-readable description of when this notification fires.
    pub description: Option<String>,
}
