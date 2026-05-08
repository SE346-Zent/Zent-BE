use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::model::responses::notifications::notification_list_response::NotificationListItem;

use super::in_app::NotificationRecord;

// ── Data types ─────────────────────────────────────────────────────────

/// An outbox entry — a pending notification delivery.
#[derive(Debug, Clone)]
pub struct OutboxRecord {
    pub outbox_id: Uuid,
    pub user_id: Uuid,
    pub notification_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
}

// ── Sync ───────────────────────────────────────────────────────────────

/// Return all undelivered notifications for a user, marking the
/// corresponding outbox entries as delivered.
///
/// Takes the full outbox and notification collections and returns the
/// list items that should be pushed to the client.
pub fn sync_outbox(
    _outbox: &mut [OutboxRecord],
    _notifs: &[NotificationRecord],
    _user_id: Uuid,
) -> Vec<NotificationListItem> {
    unimplemented!()
}

/// Delete outbox entries that have been delivered for a given user.
/// Returns the number of entries removed.
pub fn cleanup_delivered(_outbox: &mut Vec<OutboxRecord>, _user_id: Uuid) -> usize {
    unimplemented!()
}
