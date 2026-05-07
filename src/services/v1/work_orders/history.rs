use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, Order};
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    core::lookup_tables::LookupTables,
    entities::{work_order_state_history, users},
    model::responses::work_orders::history_response::WorkOrderStateHistoryEntry,
};

pub async fn decide_get_history(
    db: &DatabaseConnection,
    work_order_id: Uuid,
    luts: &LookupTables,
) -> Result<Vec<WorkOrderStateHistoryEntry>, AppError> {
    // Fetch state history rows for this work order, ordered by time
    let history_rows: Vec<(
        work_order_state_history::Model,
        Option<users::Model>,
    )> = work_order_state_history::Entity::find()
        .filter(work_order_state_history::Column::WorkOrderId.eq(work_order_id))
        .order_by(work_order_state_history::Column::ChangedAt, Order::Asc)
        .find_also_related(users::Entity)
        .all(db)
        .await?;

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

    Ok(entries)
}
