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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::in_app::NotificationRecord;
    use uuid::Uuid;
    use chrono::Utc;
    use serde_json::json;

    fn make_outbox_entry(user_id: Uuid, notification_id: Uuid, delivered: bool) -> OutboxRecord {
        OutboxRecord {
            outbox_id: Uuid::new_v4(),
            user_id,
            notification_id,
            created_at: Utc::now(),
            delivered,
        }
    }

    #[test]
    fn unit_outbox_sync_returns_pending_and_marks_delivered() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let now = Utc::now();

        let notifs = vec![NotificationRecord {
            notification_id: nid, user_id: uid, category_id: 1,
            title: "Test".into(), body: "Body".into(), data: json!({}),
            is_read: false, os_notification_id: None, created_at: now,
        }];

        let mut outbox = vec![make_outbox_entry(uid, nid, false)];
        let result = sync_outbox(&mut outbox, &notifs, uid);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].notification_id, nid.to_string());
        assert!(outbox[0].delivered);
    }

    #[test]
    fn unit_outbox_sync_skips_already_delivered() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let now = Utc::now();

        let notifs = vec![NotificationRecord {
            notification_id: nid, user_id: uid, category_id: 1,
            title: "T".into(), body: "B".into(), data: json!({}),
            is_read: false, os_notification_id: None, created_at: now,
        }];

        let mut outbox = vec![make_outbox_entry(uid, nid, true)];
        let result = sync_outbox(&mut outbox, &notifs, uid);
        assert!(result.is_empty());
    }

    #[test]
    fn unit_outbox_sync_user_scoped() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let now = Utc::now();

        let notifs = vec![NotificationRecord {
            notification_id: nid, user_id: uid_a, category_id: 1,
            title: "T".into(), body: "B".into(), data: json!({}),
            is_read: false, os_notification_id: None, created_at: now,
        }];

        let mut outbox = vec![make_outbox_entry(uid_a, nid, false)];
        let result = sync_outbox(&mut outbox, &notifs, uid_b);

        assert!(result.is_empty(), "User B must not receive user A's notifications");
        assert!(!outbox[0].delivered, "User A's entry must remain undelivered");
    }

    #[test]
    fn unit_outbox_cleanup_removes_only_delivered() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_outbox_entry(uid, Uuid::new_v4(), true),
            make_outbox_entry(uid, Uuid::new_v4(), false),
            make_outbox_entry(uid, Uuid::new_v4(), true),
        ];

        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 2);
        assert_eq!(outbox.len(), 1);
        assert!(!outbox[0].delivered);
    }

    #[test]
    fn unit_outbox_cleanup_user_scoped() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let mut outbox = vec![
            make_outbox_entry(uid_a, Uuid::new_v4(), true),
            make_outbox_entry(uid_b, Uuid::new_v4(), true),
        ];

        let removed = cleanup_delivered(&mut outbox, uid_a);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 1); // uid_b's entry remains
    }

    #[test]
    fn unit_outbox_sync_multiple_pending() {
        let uid = Uuid::new_v4();
        let now = Utc::now();
        let nid1 = Uuid::new_v4();
        let nid2 = Uuid::new_v4();

        let notifs = vec![
            NotificationRecord {
                notification_id: nid1, user_id: uid, category_id: 1,
                title: "First".into(), body: "B1".into(), data: json!({}),
                is_read: false, os_notification_id: None, created_at: now,
            },
            NotificationRecord {
                notification_id: nid2, user_id: uid, category_id: 2,
                title: "Second".into(), body: "B2".into(), data: json!({}),
                is_read: false, os_notification_id: None, created_at: now,
            },
        ];

        let mut outbox = vec![
            make_outbox_entry(uid, nid1, false),
            make_outbox_entry(uid, nid2, false),
        ];

        let result = sync_outbox(&mut outbox, &notifs, uid);
        assert_eq!(result.len(), 2);
        assert!(outbox.iter().all(|e| e.delivered));
    }
}
