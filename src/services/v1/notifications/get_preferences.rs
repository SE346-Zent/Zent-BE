use std::collections::HashMap;
use crate::model::responses::notifications::preference_response::NotificationPreferenceResponse;
use super::{NOTIFICATION_CATEGORIES, find_category_slug_by_id};

/// Build a structured list of notification preferences for the specified categories.
///
/// This pure function maps a user's raw preference overrides (category ID to
/// OS-enabled status) against a list of permitted categories. Any category missing
/// from the user's mapping defaults to `enabled` (true).
///
/// # Arguments
/// * `current_user_preferences` - A map of category IDs to their current OS-delivery status.
/// * `permitted_category_ids` - A list of category IDs relevant to the user's current role.
///
/// # Returns
/// A vector of `NotificationPreferenceResponse` objects representing the user's settings.
pub fn get_preferences(
    current_user_preferences: &HashMap<i32, bool>,
    permitted_category_ids: &[i32],
) -> Vec<NotificationPreferenceResponse> {
    permitted_category_ids.iter().filter_map(|&category_id| {
        let category_slug = find_category_slug_by_id(category_id)?;
        let index = (category_id - 1) as usize;
        let (_, display_name) = NOTIFICATION_CATEGORIES[index];
        
        let is_os_delivery_enabled = *current_user_preferences.get(&category_id).unwrap_or(&true);
        
        Some(NotificationPreferenceResponse {
            category_id,
            category_slug: category_slug.to_string(),
            category_name: display_name.to_string(),
            os_enabled: is_os_delivery_enabled,
            updated_at: None,
        })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a prefs response from a map.
    fn sut(prefs: &HashMap<i32, bool>) -> Vec<NotificationPreferenceResponse> {
        let all_ids: Vec<i32> = (1..=NOTIFICATION_CATEGORIES.len() as i32).collect();
        get_preferences(prefs, &all_ids)
    }

    // ── Empty map → everything enabled ─────────────────────────────

    #[test]
    fn test_empty_prefs_defaults_all_true() {
        let result = sut(&HashMap::new());
        assert!(!result.is_empty());
        for entry in &result {
            assert!(entry.os_enabled,
                "Category id {} should default to enabled", entry.category_id);
        }
    }

    #[test]
    fn test_empty_prefs_returns_all_categories() {
        let result = sut(&HashMap::new());
        assert_eq!(result.len(), NOTIFICATION_CATEGORIES.len());
    }

    // ── Single override ─────────────────────────────────────────────

    #[test]
    fn test_disabling_one_category() {
        let mut prefs = HashMap::new();
        prefs.insert(1, false); // disable "work_order_assigned"
        let result = sut(&prefs);

        assert!(!result[0].os_enabled, "First category should be disabled");
        // All others should still be true
        for entry in &result[1..] {
            assert!(entry.os_enabled,
                "Category id {} should remain enabled", entry.category_id);
        }
    }

    #[test]
    fn test_enabling_one_category_explicitly() {
        let mut prefs = HashMap::new();
        prefs.insert(3, true);
        let result = sut(&prefs);
        assert!(result[2].os_enabled);
    }

    // ── Multiple overrides ──────────────────────────────────────────

    #[test]
    fn test_multiple_overrides_odd_disabled_even_enabled() {
        let mut prefs = HashMap::new();
        for i in 1..=NOTIFICATION_CATEGORIES.len() {
            if i % 2 == 0 {
                prefs.insert(i as i32, false);
            }
        }
        let result = sut(&prefs);
        for entry in &result {
            if entry.category_id % 2 == 0 {
                assert!(!entry.os_enabled,
                    "Even category {} should be disabled", entry.category_id);
            } else {
                assert!(entry.os_enabled,
                    "Odd category {} should be enabled", entry.category_id);
            }
        }
    }

    #[test]
    fn test_toggle_all_off() {
        let mut prefs = HashMap::new();
        for i in 1..=NOTIFICATION_CATEGORIES.len() {
            prefs.insert(i as i32, false);
        }
        let result = sut(&prefs);
        assert!(result.iter().all(|e| !e.os_enabled));
    }

    #[test]
    fn test_toggle_all_on_explicitly() {
        let mut prefs = HashMap::new();
        for i in 1..=NOTIFICATION_CATEGORIES.len() {
            prefs.insert(i as i32, true);
        }
        let result = sut(&prefs);
        assert!(result.iter().all(|e| e.os_enabled));
    }

    // ── Category identity ───────────────────────────────────────────

    #[test]
    fn test_category_ids_are_sequential() {
        let result = sut(&HashMap::new());
        for (i, entry) in result.iter().enumerate() {
            assert_eq!(entry.category_id, (i + 1) as i32);
        }
    }

    #[test]
    fn test_category_slugs_are_correct() {
        let result = sut(&HashMap::new());
        for (i, (expected_slug, _)) in NOTIFICATION_CATEGORIES.iter().enumerate() {
            assert_eq!(result[i].category_slug, *expected_slug);
        }
    }

    #[test]
    fn test_category_names_are_correct() {
        let result = sut(&HashMap::new());
        for (i, (_, expected_name)) in NOTIFICATION_CATEGORIES.iter().enumerate() {
            assert_eq!(result[i].category_name, *expected_name);
        }
    }

    // ── updated_at behaviour ────────────────────────────────────────

    #[test]
    fn test_updated_at_is_always_none() {
        let result = sut(&HashMap::new());
        for entry in &result {
            assert!(entry.updated_at.is_none(),
                "updated_at should be None until persisted");
        }
    }

    #[test]
    fn test_updated_at_none_even_with_overrides() {
        let mut prefs = HashMap::new();
        prefs.insert(5, false);
        let result = sut(&prefs);
        for entry in &result {
            assert!(entry.updated_at.is_none());
        }
    }

    // ── Map with out-of-range keys ──────────────────────────────────

    #[test]
    fn test_out_of_range_key_is_ignored() {
        let mut prefs = HashMap::new();
        prefs.insert(999, false);
        let result = sut(&prefs);
        assert_eq!(result.len(), NOTIFICATION_CATEGORIES.len());
        // Everything should be enabled since no valid key was overridden
        assert!(result.iter().all(|e| e.os_enabled));
    }

    #[test]
    fn test_negative_key_is_ignored() {
        let mut prefs = HashMap::new();
        prefs.insert(-5, false);
        let result = sut(&prefs);
        assert!(result.iter().all(|e| e.os_enabled));
    }

    #[test]
    fn test_zero_key_is_ignored() {
        let mut prefs = HashMap::new();
        prefs.insert(0, false);
        let result = sut(&prefs);
        assert!(result.iter().all(|e| e.os_enabled));
    }

    // ── Result shape ────────────────────────────────────────────────

    #[test]
    fn test_result_length_is_invariant() {
        let mut prefs = HashMap::new();
        prefs.insert(1, false);
        prefs.insert(2, false);
        assert_eq!(sut(&prefs).len(), NOTIFICATION_CATEGORIES.len());
        assert_eq!(sut(&HashMap::new()).len(), NOTIFICATION_CATEGORIES.len());
    }

    #[test]
    fn test_deterministic_order() {
        let a = sut(&HashMap::new());
        let b = sut(&HashMap::new());
        for (ea, eb) in a.iter().zip(b.iter()) {
            assert_eq!(ea.category_id, eb.category_id);
            assert_eq!(ea.category_slug, eb.category_slug);
        }
    }

    // ── Edge: large map with garbage keys ───────────────────────────

    #[test]
    fn test_large_map_with_spam_keys() {
        let mut prefs = HashMap::new();
        for i in -100..=100 {
            prefs.insert(i, false);
        }
        // Valid categories are 1..=NOTIFICATION_CATEGORIES.len()
        let result = sut(&prefs);
        for entry in &result {
            assert!(!entry.os_enabled,
                "Valid category {} should be disabled by spam map", entry.category_id);
        }
    }
}
