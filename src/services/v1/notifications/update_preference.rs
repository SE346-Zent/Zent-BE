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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a fresh map and apply one update.
    fn apply(cat: i32, enabled: bool) -> Result<HashMap<i32, bool>, AppError> {
        let mut map = HashMap::new();
        update_preference(cat, enabled, &mut map)?;
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
        for id in 1..=8 {
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
        let err = apply(9, true).unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("9")),
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
        assert!(update_preference(2, true, &mut prefs).is_ok());
        assert!(update_preference(2, false, &mut prefs).is_ok());
        assert_eq!(prefs.len(), 1);
        assert!(!prefs.get(&2).copied().unwrap_or(true));
    }

    #[test]
    fn test_overwrite_preserves_other_categories() {
        let mut prefs = HashMap::new();
        update_preference(1, false, &mut prefs).unwrap();
        update_preference(2, false, &mut prefs).unwrap();
        update_preference(1, true, &mut prefs).unwrap(); // re-enable
        assert_eq!(prefs.len(), 2);
        assert!(prefs.get(&1).copied().unwrap_or(false));
        assert!(!prefs.get(&2).copied().unwrap_or(true));
    }

    #[test]
    fn test_three_state_flip() {
        let mut prefs = HashMap::new();
        update_preference(5, true, &mut prefs).unwrap();
        update_preference(5, false, &mut prefs).unwrap();
        update_preference(5, true, &mut prefs).unwrap();
        assert!(prefs.get(&5).copied().unwrap_or(false));
    }

    #[test]
    fn test_insert_many_categories() {
        let mut prefs = HashMap::new();
        for id in 1..=8 {
            update_preference(id, id > 4, &mut prefs).unwrap();
        }
        assert_eq!(prefs.len(), 8);
        for id in 1..=4 {
            assert!(!prefs.get(&id).copied().unwrap_or(true));
        }
        for id in 5..=8 {
            assert!(prefs.get(&id).copied().unwrap_or(false));
        }
    }

    // ── No-op guarantees ────────────────────────────────────────────

    #[test]
    fn test_error_does_not_mutate_map() {
        let mut prefs = HashMap::new();
        prefs.insert(1, true);
        let original = prefs.clone();
        let err = update_preference(99, false, &mut prefs).unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
        assert_eq!(prefs, original, "Map mutated despite error");
    }

    #[test]
    fn test_error_on_zero_does_not_mutate_empty_map() {
        let mut prefs: HashMap<i32, bool> = HashMap::new();
        let _ = update_preference(0, true, &mut prefs).unwrap_err();
        assert!(prefs.is_empty());
    }

    // ── Boundary: first and last valid ──────────────────────────────

    #[test]
    fn test_first_category_id_works() {
        let mut prefs = HashMap::new();
        update_preference(1, false, &mut prefs).unwrap();
        assert!(!prefs.get(&1).copied().unwrap_or(true));
    }

    #[test]
    fn test_last_category_id_works() {
        let mut prefs = HashMap::new();
        update_preference(8, false, &mut prefs).unwrap();
        assert!(!prefs.get(&8).copied().unwrap_or(true));
    }
}
