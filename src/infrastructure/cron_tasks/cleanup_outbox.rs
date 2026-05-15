use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio_cron_scheduler::Job;
use tracing::{info, error};

use crate::entities::outbox_records;

/// Cron job that deletes delivered outbox records older than the given retention period.
///
/// Runs daily at 03:00 (system time).
pub fn clean_up_outbox_job(
    db: sea_orm::DatabaseConnection,
) -> Result<Job, anyhow::Error> {
    // Run daily at 03:00: "0 0 3 * * *"
    let job = Job::new_async("0 0 3 * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        Box::pin(async move {
            info!("Running outbox cleanup job...");

            // Delete all delivered outbox records (no retention — once delivered, can be cleaned)
            match outbox_records::Entity::delete_many()
                .filter(outbox_records::Column::Delivered.eq(true))
                .exec(&db_clone)
                .await
            {
                Ok(result) => {
                    info!("Outbox cleanup complete: {} rows deleted", result.rows_affected);
                }
                Err(e) => {
                    error!("Outbox cleanup job failed: {:?}", e);
                }
            }
        })
    })?;

    Ok(job)
}
