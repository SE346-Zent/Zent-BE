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
