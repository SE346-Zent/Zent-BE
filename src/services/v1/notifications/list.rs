use crate::model::{
    requests::notifications::list_query::NotificationListQuery,
    responses::{
        notifications::notification_list_response::NotificationListItem,
        pagination::PaginationResponse,
    },
};
use super::NotificationRecord;

/// Assemble a paginated list of notifications for a user based on query filters.
///
/// This function performs in-memory filtering by category, sorting (newest first),
/// and pagination on a pre-fetched set of `NotificationRecord` data.
///
/// # Arguments
/// * `notification_records` - The list of raw notification records retrieved from the database.
/// * `list_query` - The query parameters for filtering and pagination.
///
/// # Returns
/// A tuple containing the list of `NotificationListItem` and the `PaginationResponse` metadata.
pub fn list_notifications(
    notification_records: &[NotificationRecord],
    list_query: &NotificationListQuery,
) -> (Vec<NotificationListItem>, PaginationResponse) {
    if let Some(category_id) = list_query.category_id {
        if !super::is_valid_category_id(category_id) {
            tracing::warn!(
                category_id = %category_id,
                error.message = "InvalidCategoryId",
                error.details = "",
                message = "Queried category ID is invalid or out of bounds"
            );
        }
    }

    let current_page = list_query.page.unwrap_or(1);
    let page_limit = list_query.limit.unwrap_or(20);
    
    let mut filtered_notifications: Vec<_> = notification_records.iter()
        .filter(|notification| {
            if let Some(category_id) = list_query.category_id {
                notification.category_id == category_id
            } else {
                true
            }
        })
        .collect();

    filtered_notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total_records = filtered_notifications.len();
    let page_offset = (current_page - 1) * page_limit;
    let paginated_items = filtered_notifications.into_iter()
        .skip(page_offset as usize)
        .take(page_limit as usize)
        .map(|notification| NotificationListItem {
            notification_id: notification.notification_id.to_string(),
            category_id: notification.category_id,
            category_name: super::find_category_slug_by_id(notification.category_id).unwrap_or("").to_string(),
            title: notification.title.clone(),
            body: notification.body.clone(),
            data: Some(notification.data.clone()),
            is_read: notification.is_read,
            created_at: notification.created_at.to_string(),
        })
        .collect();

    let total_pages = (total_records as f64 / page_limit as f64).ceil() as u64;

    tracing::info!(
        current_page = %current_page,
        page_limit = %page_limit,
        total_records = %total_records,
        reason = "ListNotificationsDecided",
        message = "Successfully paginated notifications"
    );

    (paginated_items, PaginationResponse {
        current_page: current_page as u64,
        limit: page_limit as u64,
        total_pages,
        total_records: total_records as u64,
        has_next: (current_page as u64) < total_pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc, Duration};
    use uuid::Uuid;
    use serde_json::json;

    fn make_notif(
        id: Uuid,
        cat: i32,
        created: DateTime<Utc>,
        read: bool,
    ) -> NotificationRecord {
        NotificationRecord {
            notification_id: id,
            user_id: Uuid::nil(),
            category_id: cat,
            title: "Test".into(),
            body: "Body".into(),
            data: json!({"key": "value"}),
            is_read: read,
            os_notification_id: None,
            created_at: created,
        }
    }

    fn make_query(
        page: Option<u32>,
        limit: Option<u32>,
        category_id: Option<i32>,
    ) -> NotificationListQuery {
        NotificationListQuery { page, limit, category_id }
    }

    // ── Empty list ─────────────────────────────────────────────────

    #[test]
    fn test_empty_list_returns_empty_page() {
        let (items, pag) = list_notifications(&[], &make_query(None, None, None));
        assert!(items.is_empty());
        assert_eq!(pag.total_records, 0);
        assert_eq!(pag.total_pages, 0);
        assert_eq!(pag.current_page, 1);
        assert!(!pag.has_next);
    }

    #[test]
    fn test_empty_list_with_explicit_page() {
        let (items, pag) = list_notifications(&[], &make_query(Some(2), Some(10), None));
        assert!(items.is_empty());
        assert_eq!(pag.total_records, 0);
        assert_eq!(pag.current_page, 2);
    }

    // ── Single page ─────────────────────────────────────────────────

    #[test]
    fn test_single_notification_on_page_one() {
        let now = Utc::now();
        let notifs = vec![make_notif(Uuid::new_v4(), 1, now, false)];
        let (items, pag) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items.len(), 1);
        assert_eq!(pag.total_records, 1);
        assert_eq!(pag.total_pages, 1);
        assert!(!pag.has_next);
    }

    #[test]
    fn test_exact_fit_on_one_page() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..20).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (items, pag) = list_notifications(&notifs, &make_query(None, Some(20), None));
        assert_eq!(items.len(), 20);
        assert_eq!(pag.total_pages, 1);
        assert!(!pag.has_next);
    }

    // ── Multi-page ──────────────────────────────────────────────────

    #[test]
    fn test_two_pages_correctly() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..25).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (page1, pag) = list_notifications(&notifs, &make_query(Some(1), Some(10), None));
        assert_eq!(page1.len(), 10);
        assert_eq!(pag.total_records, 25);
        assert_eq!(pag.total_pages, 3);
        assert!(pag.has_next);

        let (page2, _) = list_notifications(&notifs, &make_query(Some(2), Some(10), None));
        assert_eq!(page2.len(), 10);

        let (page3, pag3) = list_notifications(&notifs, &make_query(Some(3), Some(10), None));
        assert_eq!(page3.len(), 5);
        assert!(!pag3.has_next);
    }

    // ── Sorting (newest first) ──────────────────────────────────────

    #[test]
    fn test_newest_first_ordering() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now - Duration::hours(5), false),
            make_notif(Uuid::new_v4(), 1, now, false),
            make_notif(Uuid::new_v4(), 1, now - Duration::hours(2), false),
        ];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].created_at, now.to_string());
        assert_eq!(items[1].created_at, (now - Duration::hours(2)).to_string());
        assert_eq!(items[2].created_at, (now - Duration::hours(5)).to_string());
    }

    #[test]
    fn test_sorting_with_ties_same_timestamp() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now, false),
            make_notif(Uuid::new_v4(), 2, now, false),
            make_notif(Uuid::new_v4(), 3, now, false),
        ];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        // Should all be present; sort is stable-ish if timestamps equal
        assert_eq!(items.len(), 3);
    }

    // ── Category filter ─────────────────────────────────────────────

    #[test]
    fn test_filter_by_single_category() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now, false),
            make_notif(Uuid::new_v4(), 2, now, false),
            make_notif(Uuid::new_v4(), 1, now, false),
        ];
        let (items, pag) = list_notifications(&notifs, &make_query(None, None, Some(1)));
        assert_eq!(items.len(), 2);
        assert_eq!(pag.total_records, 2);
        for item in &items {
            assert_eq!(item.category_id, 1);
        }
    }

    #[test]
    fn test_filter_by_nonexistent_category() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now, false),
            make_notif(Uuid::new_v4(), 2, now, false),
        ];
        let (items, pag) = list_notifications(&notifs, &make_query(None, None, Some(99)));
        assert!(items.is_empty());
        assert_eq!(pag.total_records, 0);
    }

    #[test]
    fn test_no_filter_returns_all() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now, false),
            make_notif(Uuid::new_v4(), 2, now, false),
            make_notif(Uuid::new_v4(), 3, now, false),
        ];
        let (items, pag) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items.len(), 3);
        assert_eq!(pag.total_records, 3);
    }

    // ── Page boundary & overflow ────────────────────────────────────

    #[test]
    fn test_page_beyond_total_returns_empty() {
        let now = Utc::now();
        let notifs = vec![make_notif(Uuid::new_v4(), 1, now, false)];
        let (items, pag) = list_notifications(&notifs, &make_query(Some(100), Some(10), None));
        assert!(items.is_empty());
        assert_eq!(pag.total_records, 1);
        assert_eq!(pag.current_page, 100);
    }

    #[test]
    fn test_last_page_partial_fill() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..7).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (items, pag) = list_notifications(&notifs, &make_query(Some(2), Some(5), None));
        assert_eq!(items.len(), 2);
        assert_eq!(pag.total_pages, 2);
        assert!(!pag.has_next);
    }

    #[test]
    fn test_limit_zero_becomes_usize_max_surprise() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..5).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (items, pag) = list_notifications(&notifs, &make_query(None, Some(0), None));
        // unwrap_or(20) → limit 0 stays 0 → take(0) = empty, but div by zero safe
        assert!(items.is_empty());
        assert_eq!(items.len(), 0);
        assert_eq!(pag.total_records, 5);
    }

    // ── Pagination metadata ─────────────────────────────────────────

    #[test]
    fn test_has_next_edge() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..20).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();

        let (_, pag1) = list_notifications(&notifs, &make_query(Some(1), Some(10), None));
        assert!(pag1.has_next);
        assert_eq!(pag1.current_page, 1);
        assert_eq!(pag1.total_pages, 2);

        let (_, pag2) = list_notifications(&notifs, &make_query(Some(2), Some(10), None));
        assert!(!pag2.has_next);
        assert_eq!(pag2.current_page, 2);
    }

    #[test]
    fn test_limit_param_propagation() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..50).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (items, pag) = list_notifications(&notifs, &make_query(None, Some(5), None));
        assert_eq!(items.len(), 5);
        assert_eq!(pag.limit, 5);
    }

    #[test]
    fn test_default_limit_is_20() {
        let now = Utc::now();
        let notifs: Vec<_> = (0..30).map(|i| {
            make_notif(Uuid::new_v4(), 1, now + Duration::seconds(i), false)
        }).collect();
        let (items, pag) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items.len(), 20);
        assert_eq!(pag.limit, 20);
    }

    // ── Mixed read/unread ───────────────────────────────────────────

    #[test]
    fn test_mixed_read_unread_preserved() {
        let now = Utc::now();
        let notifs = vec![
            make_notif(Uuid::new_v4(), 1, now - Duration::hours(1), false),
            make_notif(Uuid::new_v4(), 1, now - Duration::hours(2), true),
            make_notif(Uuid::new_v4(), 1, now - Duration::hours(3), false),
        ];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        // Order newest first, preserving is_read
        assert!(!items[0].is_read);
        assert!(items[1].is_read);
        assert!(!items[2].is_read);
    }

    // ── No side-effects ─────────────────────────────────────────────

    #[test]
    fn test_pagination_does_not_mutate_input() {
        let now = Utc::now();
        let n1 = make_notif(Uuid::new_v4(), 1, now, false);
        let n2 = make_notif(Uuid::new_v4(), 2, now, false);
        let original = vec![n1.clone(), n2.clone()];
        let mut input = original.clone();
        let _ = list_notifications(&mut input, &make_query(None, None, None));
        assert_eq!(input.len(), original.len());
        assert_eq!(input[0].notification_id, original[0].notification_id);
    }

    // ── Response shape ──────────────────────────────────────────────

    #[test]
    fn test_notification_id_in_response_matches() {
        let now = Utc::now();
        let nid = Uuid::new_v4();
        let notifs = vec![make_notif(nid, 1, now, false)];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items[0].notification_id, nid.to_string());
    }

    #[test]
    fn test_category_name_is_resolved() {
        let now = Utc::now();
        let notifs = vec![make_notif(Uuid::new_v4(), 1, now, false)];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items[0].category_name, "work_order_assigned");
    }

    #[test]
    fn test_unknown_cat_id_falls_back_to_empty_string() {
        let now = Utc::now();
        let notifs = vec![make_notif(Uuid::new_v4(), 99, now, false)];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        assert_eq!(items[0].category_name, "");
    }

    #[test]
    fn test_data_payload_is_preserved() {
        let now = Utc::now();
        let mut n = make_notif(Uuid::new_v4(), 1, now, false);
        n.data = json!({"order": "WO-001", "status": "assigned"});
        let notifs = vec![n];
        let (items, _) = list_notifications(&notifs, &make_query(None, None, None));
        let d = items[0].data.as_ref().unwrap();
        assert_eq!(d["order"], "WO-001");
    }
}
