use uuid::Uuid;
use super::OutboxRecord;

/// Delete outbox entries that have been delivered for a given user.
/// Returns the number of entries removed.
pub fn cleanup_delivered(outbox: &mut Vec<OutboxRecord>, user_id: Uuid) -> usize {
    let initial_len = outbox.len();
    outbox.retain(|e| e.user_id != user_id || !e.delivered);
    initial_len - outbox.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_entry(id: Uuid, uid: Uuid, nid: Uuid, delivered: bool) -> OutboxRecord {
        OutboxRecord {
            outbox_id: id,
            user_id: uid,
            notification_id: nid,
            created_at: Utc::now(),
            delivered,
        }
    }

    // ── Happy path ─────────────────────────────────────────────────

    #[test]
    fn test_removes_delivered_entries_for_user() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 1);
        assert!(!outbox[0].delivered);
    }

    #[test]
    fn test_keeps_undelivered_for_user() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 0);
        assert_eq!(outbox.len(), 2);
    }

    // ── Other users unaffected ──────────────────────────────────────

    #[test]
    fn test_other_users_delivered_entries_not_removed() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid_a, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid_b, Uuid::new_v4(), true),
        ];
        let removed = cleanup_delivered(&mut outbox, uid_a);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].user_id, uid_b);
    }

    #[test]
    fn test_other_users_undelivered_also_kept() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid_a, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid_b, Uuid::new_v4(), false),
        ];
        let removed = cleanup_delivered(&mut outbox, uid_a);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].user_id, uid_b);
    }

    // ── All delivered for user → all removed ────────────────────────

    #[test]
    fn test_all_delivered_removed() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 3);
        assert!(outbox.is_empty());
    }

    // ── No delivered for user → nothing removed ─────────────────────

    #[test]
    fn test_no_delivered_for_user_removes_nothing() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 0);
        assert_eq!(outbox.len(), 2);
    }

    // ── Empty outbox ────────────────────────────────────────────────

    #[test]
    fn test_empty_outbox_returns_zero() {
        let uid = Uuid::new_v4();
        let mut outbox: Vec<OutboxRecord> = vec![];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 0);
        assert!(outbox.is_empty());
    }

    // ── User has no entries at all ──────────────────────────────────

    #[test]
    fn test_user_with_no_entries_returns_zero() {
        let uid = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), other, Uuid::new_v4(), true),
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 0);
        assert_eq!(outbox.len(), 1);
    }

    // ── Large batch ─────────────────────────────────────────────────

    #[test]
    fn test_large_batch_removes_correct_count() {
        let uid = Uuid::new_v4();
        let mut outbox: Vec<OutboxRecord> = (0..100)
            .map(|i| make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), i % 2 == 0))
            .collect();
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 50);
        assert_eq!(outbox.len(), 50);
        assert!(outbox.iter().all(|e| !e.delivered));
    }

    #[test]
    fn test_does_not_affect_undelivered_from_other_users_in_batch() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid_a, Uuid::new_v4(), true),
            make_entry(Uuid::new_v4(), uid_b, Uuid::new_v4(), false),
            make_entry(Uuid::new_v4(), uid_a, Uuid::new_v4(), false),
            make_entry(Uuid::new_v4(), uid_b, Uuid::new_v4(), true),
        ];
        let removed = cleanup_delivered(&mut outbox, uid_a);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 3);
        // uid_b entries unchanged
        let uid_b_entries: Vec<_> = outbox.iter().filter(|e| e.user_id == uid_b).collect();
        assert_eq!(uid_b_entries.len(), 2);
        assert_eq!(uid_b_entries.iter().filter(|e| e.delivered).count(), 1);
        assert_eq!(uid_b_entries.iter().filter(|e| !e.delivered).count(), 1);
    }

    // ── Idempotency ─────────────────────────────────────────────────

    #[test]
    fn test_second_call_removes_nothing() {
        let uid = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),
        ];
        let first = cleanup_delivered(&mut outbox, uid);
        assert_eq!(first, 1);
        let second = cleanup_delivered(&mut outbox, uid);
        assert_eq!(second, 0);
        assert!(outbox.is_empty());
    }

    // ── Mixed users and states ──────────────────────────────────────

    #[test]
    fn test_mixed_scenario() {
        let uid = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mut outbox = vec![
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), true),     // should be removed
            make_entry(Uuid::new_v4(), uid, Uuid::new_v4(), false),    // kept
            make_entry(Uuid::new_v4(), other, Uuid::new_v4(), true),   // kept (other user)
            make_entry(Uuid::new_v4(), other, Uuid::new_v4(), false),  // kept (other user)
        ];
        let removed = cleanup_delivered(&mut outbox, uid);
        assert_eq!(removed, 1);
        assert_eq!(outbox.len(), 3);
        // All remaining should be either other user or undelivered for uid
        assert!(!outbox.iter().any(|e| e.user_id == uid && e.delivered));
    }
}
