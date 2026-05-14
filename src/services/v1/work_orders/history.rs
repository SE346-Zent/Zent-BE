use crate::{
    core::lookup_tables::LookupTables,
    entities::{users, work_order_state_history},
    model::responses::work_orders::history_response::WorkOrderStateHistoryEntry,
};

pub fn decide_history_entries(
    history_rows: Vec<(work_order_state_history::Model, Option<users::Model>)>,
    luts: &LookupTables,
) -> Vec<WorkOrderStateHistoryEntry> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn dummy_history_model(to_status_id: i32) -> work_order_state_history::Model {
        work_order_state_history::Model {
            id: Uuid::new_v4(),
            work_order_id: Uuid::new_v4(),
            from_status_id: Some(1),
            to_status_id,
            changed_by_id: Uuid::new_v4(),
            changed_at: Utc::now(),
        }
    }

    fn dummy_user(name: &str) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            role_id: 1,
            account_status: 1,
            email: "".to_string(),
            full_name: name.to_string(),
            password_hash: "".to_string(),
            phone_number: "".to_string(),
            province: Some("".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_map_history_to_entries_success() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());
        luts.work_order_statuses.insert(2, "Assigned".to_string());

        let h1 = dummy_history_model(2);
        let u1 = dummy_user("Alice");

        let rows = vec![(h1.clone(), Some(u1))];

        let entries = decide_history_entries(rows, &luts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].changed_by, "Alice");
        assert_eq!(entries[0].from_status, Some("Pending".to_string()));
        assert_eq!(entries[0].to_status, "Assigned");
    }

    #[test]
    fn test_map_history_to_entries_missing_user() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());
        luts.work_order_statuses.insert(2, "Assigned".to_string());

        let h1 = dummy_history_model(2);
        let rows = vec![(h1.clone(), None)]; // Missing user

        let entries = decide_history_entries(rows, &luts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].changed_by, "System"); // Expect default
    }

    #[test]
    fn test_map_history_to_entries_missing_status_in_lut() {
        let luts = LookupTables::empty(); // Empty LUT, missing statuses

        let h1 = dummy_history_model(99); // Status 99 not in LUT
        let u1 = dummy_user("Alice");
        let rows = vec![(h1.clone(), Some(u1))];

        let entries = decide_history_entries(rows, &luts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from_status, None); // Couldn't resolve
        assert_eq!(entries[0].to_status, "Status 99"); // Fallback formatting
    }

    #[test]
    fn test_map_history_to_entries_empty() {
        let luts = LookupTables::empty();
        let rows = vec![];
        let entries = decide_history_entries(rows, &luts);
        assert_eq!(entries.len(), 0);
    }
}
