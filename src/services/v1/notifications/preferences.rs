use std::collections::HashMap;

use crate::{
    core::errors::AppError,
    model::responses::notifications::preference_response::NotificationPreferenceResponse,
};

/// Build a list of preference entries for every known category.
///
/// `user_prefs` is a map of `category_id → os_enabled`.  Missing entries
/// default to `true` (OS notifications enabled).
pub fn get_preferences(
    _user_prefs: &HashMap<i32, bool>,
) -> Vec<NotificationPreferenceResponse> {
    unimplemented!()
}

/// Toggle OS delivery for a single category.
///
/// Returns `Err` when the category id does not exist.
pub fn update_preference(
    _category_id: i32,
    _os_enabled: bool,
    _user_prefs: &mut HashMap<i32, bool>,
) -> Result<(), AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::v1::notifications::categories::NOTIFICATION_CATEGORIES;

    #[test]
    fn unit_preferences_default_all_enabled() {
        let prefs = get_preferences(&HashMap::new());
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
        update_preference(1, false, &mut user_prefs).unwrap();
        let prefs = get_preferences(&user_prefs);
        let cat1 = prefs.iter().find(|p| p.category_id == 1).unwrap();
        assert!(!cat1.os_enabled);

        // Toggle back on
        update_preference(1, true, &mut user_prefs).unwrap();
        let prefs = get_preferences(&user_prefs);
        let cat1 = prefs.iter().find(|p| p.category_id == 1).unwrap();
        assert!(cat1.os_enabled);
    }

    #[test]
    fn unit_preferences_invalid_category_errors() {
        let mut prefs = HashMap::new();
        let result = update_preference(0, false, &mut prefs);
        assert!(result.is_err());
        let result = update_preference(9999, true, &mut prefs);
        assert!(result.is_err());
    }

    #[test]
    fn unit_preferences_mixed_state() {
        let mut user_prefs = HashMap::new();
        update_preference(1, false, &mut user_prefs).unwrap();
        update_preference(2, false, &mut user_prefs).unwrap();
        // 3..N remain default (true)

        let prefs = get_preferences(&user_prefs);
        assert!(!prefs.iter().find(|p| p.category_id == 1).unwrap().os_enabled);
        assert!(!prefs.iter().find(|p| p.category_id == 2).unwrap().os_enabled);
        if NOTIFICATION_CATEGORIES.len() >= 3 {
            assert!(prefs.iter().find(|p| p.category_id == 3).unwrap().os_enabled);
        }
    }
}
