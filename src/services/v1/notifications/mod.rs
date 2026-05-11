pub mod list_categories;
pub mod get_preferences;
pub mod update_preference;
pub mod list;
pub mod get_detail;
pub mod mark_read;
pub mod mark_all_read;
pub mod sync_outbox;
pub mod cleanup_outbox;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Shared Constants ───────────────────────────────────────────────────

/// All available notification categories.
/// Each tuple is (slug, display_name).
pub const NOTIFICATION_CATEGORIES: &[(&str, &str)] = &[
    ("work_order_assigned", "Work Order Assigned"),
    ("work_order_started", "Work Order Started"),
    ("work_order_completed", "Work Order Completed"),
    ("work_order_rejected", "Work Order Rejected"),
    ("work_order_refusal_approved", "Refusal Approved"),
    ("work_order_scheduled", "Work Order Scheduled"),
    ("account_verified", "Account Verified"),
    ("account_locked", "Account Locked"),
];

// ── Shared Data Types ──────────────────────────────────────────────────

/// A single notification record (mirrors the MongoDB document shape).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRecord {
    pub notification_id: Uuid,
    pub user_id: Uuid,
    pub category_id: i32,
    pub title: String,
    pub body: String,
    pub data: serde_json::Value,
    pub is_read: bool,
    pub os_notification_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ── Shared Helper Functions ───────────────────────────────────────────

/// Build the full list of notification categories as a response payload.
pub fn list_categories() -> Vec<crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse> {
    NOTIFICATION_CATEGORIES.iter().enumerate().map(|(i, (slug, name))| {
        crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse {
            id: (i + 1) as i32,
            slug: slug.to_string(),
            name: name.to_string(),
            description: None,
        }
    }).collect()
}

/// Look up a category id by its slug.
pub fn find_category_id_by_slug(slug: &str) -> Option<i32> {
    NOTIFICATION_CATEGORIES.iter().position(|(s, _)| *s == slug).map(|i| (i + 1) as i32)
}

/// Look up a category slug by its id (1-based).
pub fn find_category_slug_by_id(id: i32) -> Option<&'static str> {
    if id < 1 || id > NOTIFICATION_CATEGORIES.len() as i32 {
        return None;
    }
    Some(NOTIFICATION_CATEGORIES[(id - 1) as usize].0)
}

/// Check whether a category id is valid.
pub fn is_valid_category_id(id: i32) -> bool {
    id >= 1 && id <= NOTIFICATION_CATEGORIES.len() as i32
}

/// An outbox entry — a pending notification delivery.
#[derive(Debug, Clone)]
pub struct OutboxRecord {
    pub outbox_id: Uuid,
    pub user_id: Uuid,
    pub notification_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
}
