use uuid::Uuid;
use crate::model::responses::notifications::notification_list_response::NotificationListItem;
use super::{NotificationRecord, OutboxRecord};

/// Return all undelivered notifications for a user, marking the
/// corresponding outbox entries as delivered.
///
/// Takes the full outbox and notification collections and returns the
/// list items that should be pushed to the client.
pub fn sync_outbox(
    outbox: &mut [OutboxRecord],
    notifs: &[NotificationRecord],
    user_id: Uuid,
) -> Vec<NotificationListItem> {
    let mut items = Vec::new();
    for entry in outbox.iter_mut().filter(|e| e.user_id == user_id && !e.delivered) {
        if let Some(n) = notifs.iter().find(|n| n.notification_id == entry.notification_id) {
            items.push(NotificationListItem {
                notification_id: n.notification_id.to_string(),
                category_id: n.category_id,
                title: n.title.clone(),
                body: n.body.clone(),
                is_read: n.is_read,
                created_at: n.created_at,
            });
            entry.delivered = true;
        }
    }
    items
}
