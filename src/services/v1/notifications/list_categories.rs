use crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse;
use super::list_categories as shared_list_categories;

/// Build the full list of notification categories as a response payload.
pub fn list_categories() -> Vec<NotificationCategoryResponse> {
    shared_list_categories()
}
