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
                category_name: super::find_category_slug_by_id(n.category_id).unwrap_or("").to_string(),
                title: n.title.clone(),
                body: n.body.clone(),
                data: Some(n.data.clone()),
                is_read: n.is_read,
                created_at: n.created_at.to_string(),
            });
            entry.delivered = true;
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn make_outbox(id: Uuid, uid: Uuid, nid: Uuid, delivered: bool) -> OutboxRecord {
        OutboxRecord {
            outbox_id: id,
            user_id: uid,
            notification_id: nid,
            created_at: Utc::now(),
            delivered,
        }
    }

    fn make_notif(id: Uuid, uid: Uuid, cat: i32) -> NotificationRecord {
        NotificationRecord {
            notification_id: id,
            user_id: uid,
            category_id: cat,
            title: format!("Notif {}", cat),
            body: "Body text".into(),
            data: json!({"a": 1}),
            is_read: false,
            os_notification_id: None,
            created_at: Utc::now(),
        }
    }


    // ── Happy path ─────────────────────────────────────────────────

    #[test]
    fn test_sync_returns_undelivered() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let oid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let mut outbox = vec![make_outbox(oid, uid, nid, false)];
        let items = sync_outbox(&mut outbox, &[notif], uid);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_sync_marks_entry_delivered() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let oid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let mut outbox = vec![make_outbox(oid, uid, nid, false)];
        let _ = sync_outbox(&mut outbox, &[notif], uid);
        assert!(outbox[0].delivered);
    }

    #[test]
    fn test_sync_returns_correct_item_data() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 3);
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid, nid, false)];
        let items = sync_outbox(&mut outbox, &[notif], uid);
        assert_eq!(items[0].notification_id, nid.to_string());
        assert_eq!(items[0].category_id, 3);
        assert_eq!(items[0].title, "Notif 3");
    }

    // ── Already delivered ───────────────────────────────────────────

    #[test]
    fn test_already_delivered_not_returned() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid, nid, true)];
        let items = sync_outbox(&mut outbox, &[notif], uid);
        assert!(items.is_empty());
    }

    #[test]
    fn test_delivered_entry_remains_delivered_after_sync() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid, nid, true)];
        let _ = sync_outbox(&mut outbox, &[notif], uid);
        assert!(outbox[0].delivered);
    }

    // ── Different user filtering ────────────────────────────────────

    #[test]
    fn test_other_user_outbox_not_returned() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid_a, 1);
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid_b, nid, false)];
        let items = sync_outbox(&mut outbox, &[notif], uid_a);
        assert!(items.is_empty());
    }

    #[test]
    fn test_outbox_user_only_check_not_notification_user() {
        // sync_outbox filters by outbox.user_id, not by notification.user_id.
        let uid = Uuid::new_v4();
        let other = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, other, 1);
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid, nid, false)];
        let items = sync_outbox(&mut outbox, &[notif], uid);
        // The outbox entry belongs to uid → it's processed even if the
        // notification record itself belongs to a different user.
        assert!(!items.is_empty(), "Outbox user_id match is sufficient — notification ownership is not checked");
        assert!(outbox[0].delivered);
    }

    // ── Missing notification ────────────────────────────────────────

    #[test]
    fn test_outbox_without_corresponding_notification() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        // No matching notification in notifs slice
        let mut outbox = vec![make_outbox(Uuid::new_v4(), uid, nid, false)];
        let items = sync_outbox(&mut outbox, &[], uid);
        assert!(items.is_empty());
        assert!(!outbox[0].delivered, "Should not mark delivered if notification not found");
    }

    #[test]
    fn test_partial_match_some_notifications_missing() {
        let uid = Uuid::new_v4();
        let nid1 = Uuid::new_v4();
        let nid2 = Uuid::new_v4();
        let nid3 = Uuid::new_v4();
        let notif1 = make_notif(nid1, uid, 1);
        let notif3 = make_notif(nid3, uid, 3);
        let mut outbox = vec![
            make_outbox(Uuid::new_v4(), uid, nid1, false),
            make_outbox(Uuid::new_v4(), uid, nid2, false), // no matching notif
            make_outbox(Uuid::new_v4(), uid, nid3, false),
        ];
        let items = sync_outbox(&mut outbox, &[notif1, notif3], uid);
        assert_eq!(items.len(), 2);
        assert!(outbox[0].delivered);
        assert!(!outbox[1].delivered, "Missing notif entry should not be marked delivered");
        assert!(outbox[2].delivered);
    }

    // ── Mixed states ────────────────────────────────────────────────

    #[test]
    fn test_mixed_delivered_and_undelivered() {
        let uid = Uuid::new_v4();
        let nid1 = Uuid::new_v4();
        let nid2 = Uuid::new_v4();
        let notif1 = make_notif(nid1, uid, 1);
        let notif2 = make_notif(nid2, uid, 2);
        let mut outbox = vec![
            make_outbox(Uuid::new_v4(), uid, nid1, true),  // already delivered
            make_outbox(Uuid::new_v4(), uid, nid2, false), // pending
        ];
        let items = sync_outbox(&mut outbox, &[notif1, notif2], uid);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category_id, 2);
    }

    // ── Empty outbox ────────────────────────────────────────────────

    #[test]
    fn test_empty_outbox_returns_empty() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let items = sync_outbox(&mut [], &[notif], uid);
        assert!(items.is_empty());
    }

    // ── Multiple entries for same notification ──────────────────────

    #[test]
    fn test_duplicate_outbox_entries_all_marked() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let notif = make_notif(nid, uid, 1);
        let mut outbox = vec![
            make_outbox(Uuid::new_v4(), uid, nid, false),
            make_outbox(Uuid::new_v4(), uid, nid, false),
        ];
        let items = sync_outbox(&mut outbox, &[notif], uid);
        assert_eq!(items.len(), 2, "Both duplicate entries should yield items");
        assert!(outbox.iter().all(|e| e.delivered));
    }

    // ─── Deterministic ordering ─────────────────────────────────────

    #[test]
    fn test_order_matches_outbox_insertion() {
        let uid = Uuid::new_v4();
        let nid1 = Uuid::new_v4();
        let nid2 = Uuid::new_v4();
        let n1 = make_notif(nid1, uid, 1);
        let n2 = make_notif(nid2, uid, 2);
        let mut outbox = vec![
            make_outbox(Uuid::new_v4(), uid, nid1, false),
            make_outbox(Uuid::new_v4(), uid, nid2, false),
        ];
        let items = sync_outbox(&mut outbox, &[n1, n2], uid);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].category_id, 1);
        assert_eq!(items[1].category_id, 2);
    }
}
