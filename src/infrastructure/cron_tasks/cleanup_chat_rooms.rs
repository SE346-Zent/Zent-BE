use tokio_cron_scheduler::Job;
use sea_orm::*;
use std::sync::Arc;
use chrono::{Utc, Duration};
use uuid::Uuid;
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
///
/// Uses a two-phase approach to push filtering into the DB and avoid N+1 updates:
/// 1. SELECT eligible room IDs via a join with work_orders (predicates evaluated DB-side).
/// 2. Single `update_many` to soft-delete all matched rooms in one statement.
async fn cleanup_closed_work_order_rooms(
    db: &DatabaseConnection,
    luts: &LookupTables,
) -> Result<u64, anyhow::Error> {
    let closed_id = match luts.work_order_statuses_by_name.get("Closed") {
        Some(id) => *id,
        None => return Ok(0),
    };

    let cutoff = Utc::now() - Duration::days(15);

    // Phase 1: find room IDs whose linked work order is Closed and stale
    let room_ids: Vec<Uuid> = chat_rooms::Entity::find()
        .filter(chat_rooms::Column::WorkOrderId.is_not_null())
        .filter(chat_rooms::Column::DeletedAt.is_null())
        .find_also_related(work_orders::Entity)
        .filter(work_orders::Column::WorkOrderStatusId.eq(closed_id))
        .filter(work_orders::Column::UpdatedAt.lt(cutoff))
        .all(db)
        .await?
        .into_iter()
        .map(|(room, _)| room.id)
        .collect();

    if room_ids.is_empty() {
        return Ok(0);
    }

    let count = room_ids.len() as u64;
    let now = Utc::now();

    // Phase 2: single bulk soft-delete
    chat_rooms::Entity::update_many()
        .filter(chat_rooms::Column::Id.is_in(room_ids))
        .set(chat_rooms::ActiveModel {
            deleted_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        })
        .exec(db)
        .await?;

    Ok(count)
}
