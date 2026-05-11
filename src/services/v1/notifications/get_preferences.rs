use std::collections::HashMap;
use crate::model::responses::notifications::preference_response::NotificationPreferenceResponse;
use super::{NOTIFICATION_CATEGORIES, find_category_slug_by_id};

/// Build a list of preference entries for every known category.
///
/// `user_prefs` is a map of `category_id → os_enabled`.  Missing entries
/// default to `true` (OS notifications enabled).
pub fn get_preferences(
    user_prefs: &HashMap<i32, bool>,
) -> Vec<NotificationPreferenceResponse> {
    NOTIFICATION_CATEGORIES.iter().enumerate().map(|(i, (_slug, name))| {
        let category_id = (i + 1) as i32;
        let os_enabled = *user_prefs.get(&category_id).unwrap_or(&true);
        let category_slug = find_category_slug_by_id(category_id).unwrap_or("").to_string();
        
        NotificationPreferenceResponse {
            category_id,
            category_slug,
            category_name: name.to_string(),
            os_enabled,
            updated_at: None,
        }
    }).collect()
}
