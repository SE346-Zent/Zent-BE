use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    model::{
        requests::notifications::list_query::NotificationListQuery,
        responses::{
            notifications::{
                notification_detail_response::NotificationDetailResponse,
                notification_list_response::NotificationListItem,
            },
            pagination::PaginationResponse,
        },
    },
};

// ── Data types ─────────────────────────────────────────────────────────

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

// ── List ───────────────────────────────────────────────────────────────

/// Paginate and return notifications for a user.
///
/// Results are sorted newest-first.  Ownership is NOT checked here —
/// the caller must filter by `user_id` before passing `notifs`.
pub fn list_notifications(
    _notifs: &[NotificationRecord],
    _query: &NotificationListQuery,
) -> (Vec<NotificationListItem>, PaginationResponse) {
    unimplemented!()
}

// ── Get detail ─────────────────────────────────────────────────────────

/// Fetch a single notification by id, enforcing user ownership.
pub fn get_detail(
    _notifs: &[NotificationRecord],
    _user_id: Uuid,
    _notification_id: Uuid,
) -> Result<NotificationDetailResponse, AppError> {
    unimplemented!()
}

// ── Mark read ──────────────────────────────────────────────────────────

/// Mark a single notification as read.  Returns `true` if it was
/// previously unread (i.e. this call had an effect).
pub fn mark_read(
    _notifs: &mut [NotificationRecord],
    _user_id: Uuid,
    _notification_id: Uuid,
) -> Result<bool, AppError> {
    unimplemented!()
}

/// Mark every notification for a user as read.  Returns the number
/// of notifications that were actually transitioned.
pub fn mark_all_read(_notifs: &mut [NotificationRecord], _user_id: Uuid) -> usize {
    unimplemented!()
}
