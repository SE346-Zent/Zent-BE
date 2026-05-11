use crate::model::{
    requests::notifications::list_query::NotificationListQuery,
    responses::{
        notifications::notification_list_response::NotificationListItem,
        pagination::PaginationResponse,
    },
};
use super::NotificationRecord;

/// Paginate and return notifications for a user.
///
/// Results are sorted newest-first.  Ownership is NOT checked here —
/// the caller must filter by `user_id` before passing `notifs`.
pub fn list_notifications(
    notifs: &[NotificationRecord],
    query: &NotificationListQuery,
) -> (Vec<NotificationListItem>, PaginationResponse) {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    
    let mut filtered: Vec<_> = notifs.iter()
        .filter(|n| {
            if let Some(cat_id) = query.category_id {
                n.category_id == cat_id
            } else {
                true
            }
        })
        .collect();

    filtered.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total_records = filtered.len();
    let offset = (page - 1) * limit;
    let paginated = filtered.into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(|n| NotificationListItem {
            notification_id: n.notification_id.to_string(),
            category_id: n.category_id,
            title: n.title.clone(),
            body: n.body.clone(),
            is_read: n.is_read,
            created_at: n.created_at,
        })
        .collect();

    let total_pages = (total_records as f64 / limit as f64).ceil() as i64;

    (paginated, PaginationResponse {
        page,
        limit,
        total_pages,
        total_records: total_records as i64,
    })
}
