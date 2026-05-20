pub mod get_preferences;
pub mod update_preference;
pub mod list;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Shared Constants ───────────────────────────────────────────────────

/// All available notification categories.
/// Each tuple is (slug, display_name).
pub const NOTIFICATION_CATEGORIES: &[(&str, &str)] = &[
    ("work_order_assigned", "Work Order Assigned"),
    ("about_to_start", "About to Start"),
    ("work_order_rejection_form", "Work Order Rejection Form"),
    ("add_new_part", "Add New Part"),
    ("work_order_escalation", "Work Order Escalation"),
    ("chat_message", "Chat Message"),
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

/// Look up a unique category ID by its slug.
///
/// # Arguments
/// * `category_slug` - The human-readable slug (e.g., "work_order_assigned").
///
/// # Returns
/// The 1-based integer ID if found, or `None`.
pub fn find_category_id_by_slug(category_slug: &str) -> Option<i32> {
    NOTIFICATION_CATEGORIES.iter().position(|(slug, _)| *slug == category_slug).map(|index| (index + 1) as i32)
}

/// Look up a category slug by its unique 1-based ID.
///
/// # Arguments
/// * `category_id` - The 1-based integer ID of the category.
///
/// # Returns
/// The static slug string if valid, or `None`.
pub fn find_category_slug_by_id(category_id: i32) -> Option<&'static str> {
    if category_id < 1 || category_id > NOTIFICATION_CATEGORIES.len() as i32 {
        return None;
    }
    Some(NOTIFICATION_CATEGORIES[(category_id - 1) as usize].0)
}

/// Validate whether a category ID exists within the defined system categories.
///
/// # Arguments
/// * `category_id` - The integer ID to validate.
pub fn is_valid_category_id(category_id: i32) -> bool {
    category_id >= 1 && category_id <= NOTIFICATION_CATEGORIES.len() as i32
}


