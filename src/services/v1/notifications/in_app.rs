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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::requests::notifications::list_query::NotificationListQuery;
    use uuid::Uuid;
    use chrono::Utc;
    use serde_json::json;

    fn make_record(user_id: Uuid, category_id: i32, title: &str, minutes_ago: i64) -> NotificationRecord {
        NotificationRecord {
            notification_id: Uuid::new_v4(),
            user_id,
            category_id,
            title: title.to_string(),
            body: "body".to_string(),
            data: json!({}),
            is_read: false,
            os_notification_id: None,
            created_at: Utc::now() - chrono::Duration::minutes(minutes_ago),
        }
    }

    #[test]
    fn unit_list_sorts_newest_first() {
        let uid = Uuid::new_v4();
        let records = vec![
            make_record(uid, 1, "oldest", 60),
            make_record(uid, 1, "middle", 30),
            make_record(uid, 1, "newest", 10),
        ];
        let query = NotificationListQuery { page: Some(1), limit: Some(20), category_id: None };
        let (items, pagination) = list_notifications(&records, &query);
        assert_eq!(pagination.total_records, 3);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].title, "newest");
        assert_eq!(items[1].title, "middle");
        assert_eq!(items[2].title, "oldest");
    }

    #[test]
    fn unit_list_pagination_page1_limit2() {
        let uid = Uuid::new_v4();
        let records: Vec<_> = (0..5)
            .map(|i| make_record(uid, 1, &format!("n{}", 4 - i), (i * 10) as i64))
            .collect();
        let query = NotificationListQuery { page: Some(1), limit: Some(2), category_id: None };
        let (items, pagination) = list_notifications(&records, &query);
        assert_eq!(pagination.total_records, 5);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn unit_list_pagination_page2_limit2() {
        let uid = Uuid::new_v4();
        let records: Vec<_> = (0..5)
            .map(|i| make_record(uid, 1, &format!("n{}", 4 - i), (i * 10) as i64))
            .collect();
        let query = NotificationListQuery { page: Some(2), limit: Some(2), category_id: None };
        let (items, pagination) = list_notifications(&records, &query);
        assert_eq!(pagination.total_records, 5);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn unit_list_category_filter() {
        let uid = Uuid::new_v4();
        let records = vec![
            make_record(uid, 1, "cat1", 10),
            make_record(uid, 2, "cat2", 5),
            make_record(uid, 1, "cat1-again", 1),
        ];
        let query = NotificationListQuery { page: Some(1), limit: Some(20), category_id: Some(1) };
        let (items, pagination) = list_notifications(&records, &query);
        assert_eq!(pagination.total_records, 2);
        assert!(items.iter().all(|i| i.category_id == 1));
    }

    #[test]
    fn unit_list_default_page_and_limit() {
        let uid = Uuid::new_v4();
        let records: Vec<_> = (0..30).map(|i| make_record(uid, 1, &format!("n{}", i), i as i64)).collect();
        let query = NotificationListQuery { page: None, limit: None, category_id: None };
        let (items, _) = list_notifications(&records, &query);
        assert_eq!(items.len(), 20); // default limit
    }

    #[test]
    fn unit_get_detail_owner_can_access() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut record = make_record(uid, 1, "test", 5);
        record.notification_id = nid;
        let records = vec![record];
        let result = get_detail(&records, uid, nid);
        assert!(result.is_ok());
        let detail = result.unwrap();
        assert_eq!(detail.notification_id, nid.to_string());
        assert_eq!(detail.title, "test");
    }

    #[test]
    fn unit_get_detail_wrong_user_returns_not_found() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut record = make_record(uid_a, 1, "test", 5);
        record.notification_id = nid;
        let records = vec![record];
        let result = get_detail(&records, uid_b, nid);
        assert!(result.is_err());
    }

    #[test]
    fn unit_get_detail_nonexistent_returns_not_found() {
        let uid = Uuid::new_v4();
        let records: Vec<NotificationRecord> = vec![];
        let result = get_detail(&records, uid, Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn unit_mark_read_transitions_unread_to_read() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut record = make_record(uid, 1, "test", 5);
        record.notification_id = nid;
        record.is_read = false;
        let mut records = vec![record];

        let changed = mark_read(&mut records, uid, nid).unwrap();
        assert!(changed);
        assert!(records[0].is_read);
    }

    #[test]
    fn unit_mark_read_already_read_is_idempotent() {
        let uid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut record = make_record(uid, 1, "test", 5);
        record.notification_id = nid;
        record.is_read = true;
        let mut records = vec![record];

        let changed = mark_read(&mut records, uid, nid).unwrap();
        assert!(!changed);
        assert!(records[0].is_read);
    }

    #[test]
    fn unit_mark_read_cross_user_errors() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let mut record = make_record(uid_a, 1, "test", 5);
        record.notification_id = nid;
        let mut records = vec![record];

        let result = mark_read(&mut records, uid_b, nid);
        assert!(result.is_err());
    }

    #[test]
    fn unit_mark_all_read_counts_transitions() {
        let uid = Uuid::new_v4();
        let mut records: Vec<_> = (0..10)
            .map(|i| {
                let mut r = make_record(uid, 1, &format!("n{}", i), i as i64);
                r.is_read = i >= 5; // first 5 unread, last 5 read
                r
            })
            .collect();
        let count = mark_all_read(&mut records, uid);
        assert_eq!(count, 5);
        assert!(records.iter().all(|n| n.is_read));
    }

    #[test]
    fn unit_mark_all_read_when_all_already_read() {
        let uid = Uuid::new_v4();
        let mut records: Vec<_> = (0..3)
            .map(|i| {
                let mut r = make_record(uid, 1, &format!("n{}", i), i as i64);
                r.is_read = true;
                r
            })
            .collect();
        let count = mark_all_read(&mut records, uid);
        assert_eq!(count, 0);
    }

    #[test]
    fn unit_mark_all_read_user_scoped() {
        let uid_a = Uuid::new_v4();
        let uid_b = Uuid::new_v4();
        let mut records = vec![
            make_record(uid_a, 1, "a1", 10),
            make_record(uid_a, 1, "a2", 5),
            make_record(uid_b, 1, "b1", 1),
        ];
        let count = mark_all_read(&mut records, uid_a);
        assert_eq!(count, 2);
        // User B's notification remains unread
        assert!(!records[2].is_read);
    }
}
