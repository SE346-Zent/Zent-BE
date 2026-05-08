use crate::{
    core::lookup_tables::LookupTables,
    entities::{work_order_state_history, users},
    model::responses::work_orders::history_response::WorkOrderStateHistoryEntry,
};

/// Pure transformation: maps raw history rows + user data into API response entries.
/// The caller (handler) is responsible for fetching the rows from the database.
pub fn decide_get_history(
    history_rows: Vec<(
        work_order_state_history::Model,
        Option<users::Model>,
    )>,
    luts: &LookupTables,
) -> Vec<WorkOrderStateHistoryEntry> {
    let mut entries = Vec::with_capacity(history_rows.len());

    for (history, user) in history_rows {
        let from_status = history.from_status_id
            .and_then(|id| luts.work_order_statuses.get(&id).cloned());
        let to_status = luts.work_order_statuses
            .get(&history.to_status_id)
            .cloned()
            .unwrap_or_else(|| format!("Status {}", history.to_status_id));
        let changed_by = user.map(|u| u.full_name).unwrap_or_else(|| "System".to_string());

        entries.push(WorkOrderStateHistoryEntry {
            id: history.id,
            changed_by,
            from_status,
            to_status,
            changed_at: history.changed_at,
        });
    }

    entries
}
