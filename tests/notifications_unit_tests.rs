use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use zent_be::services::v1::notifications::{
    categories::{self, NOTIFICATION_CATEGORIES},
    preferences,
    in_app::{self, NotificationRecord},
    outbox::{self, OutboxRecord},
};
use zent_be::model::requests::notifications::list_query::NotificationListQuery;

// =====================================================================
// Categories unit tests
// =====================================================================

#[test]
fn unit_categories_list_returns_all() {
    let cats = categories::list_categories();
    assert_eq!(cats.len(), NOTIFICATION_CATEGORIES.len());
    for (i, cat) in cats.iter().enumerate() {
        assert_eq!(cat.id, (i + 1) as i32);
        assert!(!cat.name.is_empty());
        assert!(!cat.slug.is_empty());
    }
}

#[test]
fn unit_categories_find_by_slug_valid() {
    let id = categories::find_category_id_by_slug("work_order_assigned");
    assert!(id.is_some());
    assert_eq!(id.unwrap(), 1);
}

#[test]
fn unit_categories_find_by_slug_invalid() {
    let id = categories::find_category_id_by_slug("nonexistent");
    assert!(id.is_none());
}

#[test]
fn unit_categories_find_by_id_valid() {
    let slug = categories::find_category_slug_by_id(1);
    assert_eq!(slug, Some("work_order_assigned"));
}

#[test]
fn unit_categories_find_by_id_invalid() {
    assert!(categories::find_category_slug_by_id(0).is_none());
    assert!(categories::find_category_slug_by_id(9999).is_none());
}

#[test]
fn unit_categories_is_valid() {
    assert!(categories::is_valid_category_id(1));
    assert!(categories::is_valid_category_id(NOTIFICATION_CATEGORIES.len() as i32));
    assert!(!categories::is_valid_category_id(0));
    assert!(!categories::is_valid_category_id(NOTIFICATION_CATEGORIES.len() as i32 + 1));
}

// =====================================================================
// Preferences unit tests
// =====================================================================

#[test]
fn unit_preferences_default_all_enabled() {
    let prefs = preferences::get_preferences(&HashMap::new());
    assert_eq!(prefs.len(), NOTIFICATION_CATEGORIES.len());
    for p in &prefs {
        assert!(p.os_enabled, "Category {} must default to enabled", p.category_id);
        assert!(p.category_id > 0);
        assert!(!p.category_name.is_empty());
        assert!(!p.category_slug.is_empty());
    }
}

#[test]
fn unit_preferences_toggle_off_then_on() {
    let mut user_prefs = HashMap::new();

    // Toggle off
    preferences::update_preference(1, false, &mut user_prefs).unwrap();
    let prefs = preferences::get_preferences(&user_prefs);
    let cat1 = prefs.iter().find(|p| p.category_id == 1).unwrap();
    assert!(!cat1.os_enabled);

    // Toggle back on
    preferences::update_preference(1, true, &mut user_prefs).unwrap();
    let prefs = preferences::get_preferences(&user_prefs);
    let cat1 = prefs.iter().find(|p| p.category_id == 1).unwrap();
    assert!(cat1.os_enabled);
}

#[test]
fn unit_preferences_invalid_category_errors() {
    let mut prefs = HashMap::new();
    let result = preferences::update_preference(0, false, &mut prefs);
    assert!(result.is_err());
    let result = preferences::update_preference(9999, true, &mut prefs);
    assert!(result.is_err());
}

#[test]
fn unit_preferences_mixed_state() {
    let mut user_prefs = HashMap::new();
    preferences::update_preference(1, false, &mut user_prefs).unwrap();
    preferences::update_preference(2, false, &mut user_prefs).unwrap();
    // 3..N remain default (true)

    let prefs = preferences::get_preferences(&user_prefs);
    assert!(!prefs.iter().find(|p| p.category_id == 1).unwrap().os_enabled);
    assert!(!prefs.iter().find(|p| p.category_id == 2).unwrap().os_enabled);
    if NOTIFICATION_CATEGORIES.len() >= 3 {
        assert!(prefs.iter().find(|p| p.category_id == 3).unwrap().os_enabled);
    }
}

// =====================================================================
// In-app notification unit tests
// =====================================================================

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
    let (items, pagination) = in_app::list_notifications(&records, &query);
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
    let (items, pagination) = in_app::list_notifications(&records, &query);
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
    let (items, pagination) = in_app::list_notifications(&records, &query);
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
    let (items, pagination) = in_app::list_notifications(&records, &query);
    assert_eq!(pagination.total_records, 2);
    assert!(items.iter().all(|i| i.category_id == 1));
}

#[test]
fn unit_list_default_page_and_limit() {
    let uid = Uuid::new_v4();
    let records: Vec<_> = (0..30).map(|i| make_record(uid, 1, &format!("n{}", i), i as i64)).collect();
    let query = NotificationListQuery { page: None, limit: None, category_id: None };
    let (items, _) = in_app::list_notifications(&records, &query);
    assert_eq!(items.len(), 20); // default limit
}

#[test]
fn unit_get_detail_owner_can_access() {
    let uid = Uuid::new_v4();
    let nid = Uuid::new_v4();
    let mut record = make_record(uid, 1, "test", 5);
    record.notification_id = nid;
    let records = vec![record];
    let result = in_app::get_detail(&records, uid, nid);
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
    let result = in_app::get_detail(&records, uid_b, nid);
    assert!(result.is_err());
}

#[test]
fn unit_get_detail_nonexistent_returns_not_found() {
    let uid = Uuid::new_v4();
    let records: Vec<NotificationRecord> = vec![];
    let result = in_app::get_detail(&records, uid, Uuid::new_v4());
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

    let changed = in_app::mark_read(&mut records, uid, nid).unwrap();
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

    let changed = in_app::mark_read(&mut records, uid, nid).unwrap();
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

    let result = in_app::mark_read(&mut records, uid_b, nid);
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
    let count = in_app::mark_all_read(&mut records, uid);
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
    let count = in_app::mark_all_read(&mut records, uid);
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
    let count = in_app::mark_all_read(&mut records, uid_a);
    assert_eq!(count, 2);
    // User B's notification remains unread
    assert!(!records[2].is_read);
}

// =====================================================================
// Outbox unit tests
// =====================================================================

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
    let result = outbox::sync_outbox(&mut outbox, &notifs, uid);

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
    let result = outbox::sync_outbox(&mut outbox, &notifs, uid);
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
    let result = outbox::sync_outbox(&mut outbox, &notifs, uid_b);

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

    let removed = outbox::cleanup_delivered(&mut outbox, uid);
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

    let removed = outbox::cleanup_delivered(&mut outbox, uid_a);
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

    let result = outbox::sync_outbox(&mut outbox, &notifs, uid);
    assert_eq!(result.len(), 2);
    assert!(outbox.iter().all(|e| e.delivered));
}
