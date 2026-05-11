use std::collections::HashMap;
use crate::core::errors::AppError;
use super::is_valid_category_id;

/// Toggle OS delivery for a single category.
///
/// Returns `Err` when the category id does not exist.
pub fn update_preference(
    category_id: i32,
    os_enabled: bool,
    user_prefs: &mut HashMap<i32, bool>,
) -> Result<(), AppError> {
    if !is_valid_category_id(category_id) {
        return Err(AppError::BadRequest(format!("Invalid notification category ID: {}", category_id)));
    }

    user_prefs.insert(category_id, os_enabled);
    Ok(())
}
