use crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse;

/// All available notification categories.
/// Each tuple is (slug, display_name).
pub const NOTIFICATION_CATEGORIES: &[(&str, &str)] = &[
    ("work_order_assigned", "Work Order Assigned"),
    ("work_order_started", "Work Order Started"),
    ("work_order_completed", "Work Order Completed"),
    ("work_order_rejected", "Work Order Rejected"),
    ("work_order_refusal_approved", "Refusal Approved"),
    ("work_order_scheduled", "Work Order Scheduled"),
    ("account_verified", "Account Verified"),
    ("account_locked", "Account Locked"),
];

/// Build the full list of notification categories as a response payload.
pub fn list_categories() -> Vec<NotificationCategoryResponse> {
    unimplemented!()
}

/// Look up a category id by its slug.
pub fn find_category_id_by_slug(_slug: &str) -> Option<i32> {
    unimplemented!()
}

/// Look up a category slug by its id (1-based).
pub fn find_category_slug_by_id(_id: i32) -> Option<&'static str> {
    unimplemented!()
}

/// Check whether a category id is valid.
pub fn is_valid_category_id(_id: i32) -> bool {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_categories_list_returns_all() {
        let cats = list_categories();
        assert_eq!(cats.len(), NOTIFICATION_CATEGORIES.len());
        for (i, cat) in cats.iter().enumerate() {
            assert_eq!(cat.id, (i + 1) as i32);
            assert!(!cat.name.is_empty());
            assert!(!cat.slug.is_empty());
        }
    }

    #[test]
    fn unit_categories_find_by_slug_valid() {
        let id = find_category_id_by_slug("work_order_assigned");
        assert!(id.is_some());
        assert_eq!(id.unwrap(), 1);
    }

    #[test]
    fn unit_categories_find_by_slug_invalid() {
        let id = find_category_id_by_slug("nonexistent");
        assert!(id.is_none());
    }

    #[test]
    fn unit_categories_find_by_id_valid() {
        let slug = find_category_slug_by_id(1);
        assert_eq!(slug, Some("work_order_assigned"));
    }

    #[test]
    fn unit_categories_find_by_id_invalid() {
        assert!(find_category_slug_by_id(0).is_none());
        assert!(find_category_slug_by_id(9999).is_none());
    }

    #[test]
    fn unit_categories_is_valid() {
        assert!(is_valid_category_id(1));
        assert!(is_valid_category_id(NOTIFICATION_CATEGORIES.len() as i32));
        assert!(!is_valid_category_id(0));
        assert!(!is_valid_category_id(NOTIFICATION_CATEGORIES.len() as i32 + 1));
    }
}
