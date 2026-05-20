use crate::{
    core::lookup_tables::LookupTables,
    entities::{users, work_order_state_history, work_orders, work_order_closing_forms},
    model::responses::work_orders::history_response::{WorkOrderStateHistoryEntry, WorkOrderHistoryDetail, ClosingFormEntry, ComplaintEntry},
};

/// Transform raw state-history records and associated users into human-readable response entries.
///
/// This function resolves status names from lookup tables and provides fallback 
/// values for missing users or status definitions.

/// Pure logic: maps raw state-history rows and lookup tables into response entries.
/// The handler is responsible for fetching the rows from the database.
pub fn decide_get_history(
    work_order_history_tuples: Vec<(work_order_state_history::Model, Option<users::Model>)>,
    lookup_tables: &LookupTables,
) -> Vec<WorkOrderStateHistoryEntry> {
    let mut history_entries = Vec::with_capacity(work_order_history_tuples.len());

    for (state_history, changing_user) in work_order_history_tuples {
        let from_status = state_history.from_status_id
            .and_then(|id| lookup_tables.work_order_statuses.get(&id).cloned());
        let to_status = lookup_tables.work_order_statuses
            .get(&state_history.to_status_id)
            .cloned()
            .unwrap_or_else(|| format!("Status {}", state_history.to_status_id));
        let changed_by = changing_user.map(|u| u.full_name).unwrap_or_else(|| "System".to_string());

        history_entries.push(WorkOrderStateHistoryEntry {
            id: state_history.id,
            changed_by,
            from_status,
            to_status,
            changed_at: state_history.changed_at,
        });
    }

    history_entries
}

/// Pure logic: maps history rows, work order, and optional closing form into the full
/// history detail response (state transitions + closing form + complaint).
pub fn decide_get_history_detail(
    history_rows: Vec<(work_order_state_history::Model, Option<users::Model>)>,
    luts: &LookupTables,
    wo: work_orders::Model,
    closing_form: Option<work_order_closing_forms::Model>,
) -> WorkOrderHistoryDetail {
    let state_history = decide_get_history(history_rows, luts);

    let closing_form = closing_form.map(|cf| ClosingFormEntry {
        id: cf.id,
        mtm: cf.mtm,
        serial_number: cf.serial_number,
        diagnosis: cf.diagnosis,
        signature_file_name: cf.signature_file_name,
        created_at: cf.created_at,
    });

    let complaint = wo.customer_complaint.map(|message| ComplaintEntry {
        message,
        submitted_at: wo.customer_complaint_at.unwrap_or(wo.updated_at),
    });

    WorkOrderHistoryDetail {
        state_history,
        closing_form,
        complaint,
    }
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
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_map_history_to_entries_success() {
        let mut lookup_tables = LookupTables::empty();
        lookup_tables.work_order_statuses.insert(1, "Pending".to_string());
        lookup_tables.work_order_statuses.insert(2, "Assigned".to_string());

        let h1 = dummy_history_model(2);
        let u1 = dummy_user("Alice");

        let rows = vec![(h1.clone(), Some(u1))];

        let entries = decide_get_history(rows, &lookup_tables);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].changed_by, "Alice");
        assert_eq!(entries[0].from_status, Some("Pending".to_string()));
        assert_eq!(entries[0].to_status, "Assigned");
    }

    #[test]
    fn test_map_history_to_entries_missing_user() {
        let mut lookup_tables = LookupTables::empty();
        lookup_tables.work_order_statuses.insert(1, "Pending".to_string());
        lookup_tables.work_order_statuses.insert(2, "Assigned".to_string());

        let h1 = dummy_history_model(2);
        let rows = vec![(h1.clone(), None)]; // Missing user

        let entries = decide_get_history(rows, &lookup_tables);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].changed_by, "System"); // Expect default
    }

    #[test]
    fn test_map_history_to_entries_missing_status_in_lut() {
        let lookup_tables = LookupTables::empty(); // Empty LUT, missing statuses

        let h1 = dummy_history_model(99); // Status 99 not in LUT
        let u1 = dummy_user("Alice");
        let rows = vec![(h1.clone(), Some(u1))];

        let entries = decide_get_history(rows, &lookup_tables);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].from_status, None); // Couldn't resolve
        assert_eq!(entries[0].to_status, "Status 99"); // Fallback formatting
    }

    #[test]
    fn test_map_history_to_entries_empty() {
        let lookup_tables = LookupTables::empty();
        let rows = vec![];
        let entries = decide_get_history(rows, &lookup_tables);
        assert_eq!(entries.len(), 0);
    }
}       
