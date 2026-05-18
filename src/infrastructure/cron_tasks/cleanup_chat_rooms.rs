use tokio_cron_scheduler::Job;
use sea_orm::*;
use std::sync::Arc;
use chrono::{Utc, Duration};
use tracing::{info, error};

use crate::core::lookup_tables::LookupTables;
use crate::entities::{chat_rooms, work_orders};

pub fn build_cleanup_chat_rooms_job(
    db: DatabaseConnection,
    luts: Arc<LookupTables>,
) -> Result<Job, anyhow::Error> {
    // Run once per day at midnight: "0 0 0 * * *"
    let job = Job::new_async("0 0 0 * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let luts_clone = luts.clone();
        Box::pin(async move {
            info!("Running chat room cleanup job (15-day post-WO-close soft delete)...");
            match cleanup_closed_work_order_rooms(&db_clone, &luts_clone).await {
                Ok(count) => {
                    if count > 0 {
                        info!("Soft-deleted {} chat rooms linked to closed work orders", count);
                    }
                }
                Err(e) => {
                    error!("Error in chat room cleanup job: {:?}", e);
                }
            }
        })
    })?;
    Ok(job)
}

/// Soft-deletes chat rooms whose linked work order was closed more than 15 days ago.
async fn cleanup_closed_work_order_rooms(
    db: &DatabaseConnection,
    luts: &LookupTables,
) -> Result<u64, anyhow::Error> {
    let closed_id = match luts.work_order_statuses_by_name.get("Closed") {
        Some(id) => *id,
        None => return Ok(0),
    };

    let cutoff = Utc::now() - Duration::days(15);

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

    Ok(count)
}
