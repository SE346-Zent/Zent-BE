use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use chrono::{Utc, Duration};
use crate::entities::{chat_rooms, work_orders};
use crate::core::lookup_tables::LookupTables;

/// Soft-deletes chat rooms whose linked work order was closed more than 15 days ago.
pub async fn cleanup_closed_work_order_rooms(
    db: &DatabaseConnection,
    luts: &LookupTables,
) -> Result<u64, anyhow::Error> {
    let closed_id = match luts.work_order_statuses_by_name.get("Closed") {
        Some(id) => *id,
        None => return Ok(0),
    };

    let cutoff = Utc::now() - Duration::days(15);

    // Find rooms linked to closed work orders older than cutoff
    let rooms: Vec<chat_rooms::Model> = chat_rooms::Entity::find()
        .filter(chat_rooms::Column::WorkOrderId.is_not_null())
        .filter(chat_rooms::Column::DeletedAt.is_null())
        .find_also_related(work_orders::Entity)
        .all(db)
        .await?
        .into_iter()
        .filter_map(|(room, wo)| {
            if let Some(wo) = wo {
                if wo.work_order_status_id == closed_id && wo.updated_at < cutoff {
                    return Some(room);
                }
            }
            None
        })
        .collect();

    let count = rooms.len() as u64;
    let now = Utc::now();

    for room in rooms {
        let mut active: chat_rooms::ActiveModel = room.into();
        active.deleted_at = Set(Some(now));
        active.updated_at = Set(Some(now));
        active.update(db).await?;
    }

    if count > 0 {
        tracing::info!("Soft-deleted {} chat rooms linked to closed work orders older than 15 days", count);
    }

    Ok(count)
}
