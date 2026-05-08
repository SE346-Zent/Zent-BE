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
