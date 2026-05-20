use std::collections::HashMap;
use crate::core::errors::AppError;

/// Update the user's OS notification delivery preference for a specific category.
///
/// This pure function validates that the requested category is within the set of
/// categories permitted for the user's specific role before updating the
/// preference mapping.
///
/// # Arguments
/// * `target_category_id` - The ID of the notification category to update.
/// * `is_os_delivery_enabled` - Boolean indicating if push notifications should be enabled.
/// * `current_user_preferences` - A mutable reference to the user's existing category preferences.
/// * `permitted_category_ids` - A slice of category IDs allowed for the user's role.
///
/// # Returns
/// A result indicating success (`Ok(())`) or a `BadRequest` error if the category is not permitted.
pub fn update_preference(
    target_category_id: i32,
    is_os_delivery_enabled: bool,
    current_user_preferences: &mut HashMap<i32, bool>,
    permitted_category_ids: &[i32],
) -> Result<(), AppError> {
    if !permitted_category_ids.contains(&target_category_id) {
        return Err(AppError::BadRequest(format!("Invalid notification category ID for your role: {}", target_category_id)));
    }

    current_user_preferences.insert(target_category_id, is_os_delivery_enabled);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a fresh map and apply one update.
    fn apply(cat: i32, enabled: bool) -> Result<HashMap<i32, bool>, AppError> {
        let mut map = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        update_preference(cat, enabled, &mut map, &all_ids)?;
        Ok(map)
    }

    // ── Valid categories ────────────────────────────────────────────

    #[test]
    fn test_enable_valid_category() {
        let result = apply(1, true).unwrap();
        assert!(result.get(&1).copied().unwrap_or(false));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_disable_valid_category() {
        let result = apply(4, false).unwrap();
        assert!(!result.contains_key(&3));
        assert!(!result.get(&4).copied().unwrap_or(true));
    }

    #[test]
    fn test_all_valid_ids_accepted() {
        for id in 1..=4 {
            let result = apply(id, id % 2 == 0).unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(*result.get(&id).unwrap(), id % 2 == 0);
        }
    }

    // ── Invalid categories → Err ────────────────────────────────────

    #[test]
    fn test_id_zero_rejected() {
        let err = apply(0, true).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("0")),
            _ => panic!("Expected BadRequest, got {:?}", err),
        }
    }

    #[test]
    fn test_id_negative_rejected() {
        let err = apply(-1, false).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("-1")),
            _ => panic!("Expected BadRequest, got {:?}", err),
        }
    }

    #[test]
    fn test_id_just_above_max_rejected() {
        let err = apply(5, true).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("5")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_id_way_above_max_rejected() {
        let err = apply(999, false).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("999")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_i32_min_rejected() {
        let err = apply(i32::MIN, true).unwrap_err();
        match err {
            AppError::BadRequest(_) => {},
            _ => panic!("Expected BadRequest"),
        }
    }

    #[test]
    fn test_i32_max_rejected() {
        let err = apply(i32::MAX, false).unwrap_err();
        match err {
            AppError::BadRequest(_) => {},
            _ => panic!("Expected BadRequest"),
        }
    }

    // ── Idempotency & overwrite ─────────────────────────────────────

    #[test]
    fn test_overwrite_same_category_twice() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        assert!(update_preference(2, true, &mut prefs, &all_ids).is_ok());
        assert!(update_preference(2, false, &mut prefs, &all_ids).is_ok());
        assert_eq!(prefs.len(), 1);
        assert!(!prefs.get(&2).copied().unwrap_or(true));
    }

    #[test]
    fn test_overwrite_preserves_other_categories() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        update_preference(1, false, &mut prefs, &all_ids).unwrap();
        update_preference(2, false, &mut prefs, &all_ids).unwrap();
        update_preference(1, true, &mut prefs, &all_ids).unwrap(); // re-enable
        assert_eq!(prefs.len(), 2);
        assert!(prefs.get(&1).copied().unwrap_or(false));
        assert!(!prefs.get(&2).copied().unwrap_or(true));
    }

    #[test]
    fn test_three_state_flip() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        update_preference(3, true, &mut prefs, &all_ids).unwrap();
        update_preference(3, false, &mut prefs, &all_ids).unwrap();
        update_preference(3, true, &mut prefs, &all_ids).unwrap();
        assert!(prefs.get(&3).copied().unwrap_or(false));
    }

    #[test]
    fn test_insert_many_categories() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        for id in 1..=4 {
            update_preference(id, id > 2, &mut prefs, &all_ids).unwrap();
        }
        assert_eq!(prefs.len(), 4);
        for id in 1..=2 {
            assert!(!prefs.get(&id).copied().unwrap_or(true));
        }
        for id in 3..=4 {
            assert!(prefs.get(&id).copied().unwrap_or(false));
        }
    }

    // ── No-op guarantees ────────────────────────────────────────────

    #[test]
    fn test_error_does_not_mutate_map() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        prefs.insert(1, true);
        let original = prefs.clone();
        let err = update_preference(99, false, &mut prefs, &all_ids).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
        assert_eq!(prefs, original, "Map mutated despite error");
    }

    #[test]
    fn test_error_on_zero_does_not_mutate_empty_map() {
        let mut prefs: HashMap<i32, bool> = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        let _ = update_preference(0, true, &mut prefs, &all_ids).unwrap_err();
        assert!(prefs.is_empty());
    }

    // ── Boundary: first and last valid ──────────────────────────────

    #[test]
    fn test_first_category_id_works() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        update_preference(1, false, &mut prefs, &all_ids).unwrap();
        assert!(!prefs.get(&1).copied().unwrap_or(true));
    }

    #[test]
    fn test_last_category_id_works() {
        let mut prefs = HashMap::new();
        let all_ids: Vec<i32> = (1..=4).collect();
        update_preference(4, false, &mut prefs, &all_ids).unwrap();
        assert!(!prefs.get(&4).copied().unwrap_or(true));
    }
}
