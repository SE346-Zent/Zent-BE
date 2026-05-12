use crate::model::responses::notifications::notification_category_response::NotificationCategoryResponse;
use super::list_categories as shared_list_categories;

/// Build the full list of notification categories as a response payload.
pub fn list_categories() -> Vec<NotificationCategoryResponse> {
    shared_list_categories()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::v1::notifications::NOTIFICATION_CATEGORIES;

    /// The helper used by the production path.
    fn sut() -> Vec<NotificationCategoryResponse> {
        list_categories()
    }

    // ── Happy path ────────────────────────────────────────────────

    #[test]
    fn test_returns_all_categories() {
        let result = sut();
        assert_eq!(result.len(), NOTIFICATION_CATEGORIES.len(),
            "Must return exactly one entry per defined category");
    }

    #[test]
    fn test_ids_are_one_based_and_sequential() {
        let result = sut();
        for (i, cat) in result.iter().enumerate() {
            assert_eq!(cat.id, (i + 1) as i32,
                "Category at index {} should have id {}", i, i + 1);
        }
    }

    #[test]
    fn test_slugs_match_definition_order() {
        let result = sut();
        for (i, (expected_slug, _)) in NOTIFICATION_CATEGORIES.iter().enumerate() {
            assert_eq!(result[i].slug, *expected_slug,
                "Category {} slug mismatch", i);
        }
    }

    #[test]
    fn test_names_match_definition_order() {
        let result = sut();
        for (i, (_, expected_name)) in NOTIFICATION_CATEGORIES.iter().enumerate() {
            assert_eq!(result[i].name, *expected_name,
                "Category {} name mismatch", i);
        }
    }

    #[test]
    fn test_every_slug_is_non_empty() {
        for cat in sut() {
            assert!(!cat.slug.is_empty(), "Category id {} has empty slug", cat.id);
        }
    }

    #[test]
    fn test_every_name_is_non_empty() {
        for cat in sut() {
            assert!(!cat.name.is_empty(), "Category id {} has empty name", cat.id);
        }
    }

    #[test]
    fn test_no_duplicate_ids() {
        let result = sut();
        let mut ids: Vec<i32> = result.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), result.len(), "Duplicate category ids detected");
    }

    #[test]
    fn test_no_duplicate_slugs() {
        let result = sut();
        let mut slugs: Vec<&str> = result.iter().map(|c| c.slug.as_str()).collect();
        slugs.sort();
        let deduped = {
            let mut v = slugs.clone();
            v.dedup();
            v
        };
        assert_eq!(deduped.len(), slugs.len(), "Duplicate category slugs detected");
    }

    #[test]
    fn test_description_is_always_none_currently() {
        for cat in sut() {
            assert!(cat.description.is_none(),
                "Category id {} unexpectedly has a description", cat.id);
        }
    }

    // ── Known-slug smoke tests ──────────────────────────────────────

    #[test]
    fn test_work_order_assigned_is_first() {
        let result = sut();
        assert_eq!(result[0].slug, "work_order_assigned");
        assert_eq!(result[0].name, "Work Order Assigned");
    }

    #[test]
    fn test_account_locked_is_last() {
        let result = sut();
        let last = result.last().unwrap();
        assert_eq!(last.slug, "account_locked");
        assert_eq!(last.name, "Account Locked");
    }

    #[test]
    fn test_every_slug_uses_snake_case() {
        for cat in sut() {
            assert!(cat.slug.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "Slug '{}' is not snake_case", cat.slug);
        }
    }

    // ── Determinism ─────────────────────────────────────────────────

    #[test]
    fn test_result_is_deterministic() {
        let a = sut();
        let b = sut();
        assert_eq!(a, b, "Two calls returned different results");
    }

    #[test]
    fn test_matches_length_of_const() {
        assert_eq!(sut().len(), NOTIFICATION_CATEGORIES.len());
    }
}
