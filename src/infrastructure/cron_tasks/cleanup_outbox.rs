use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio_cron_scheduler::Job;
use tracing::{info, error};

use crate::entities::outbox_records;

/// Create a new cron job that periodically deletes delivered outbox records.
///
/// The job runs daily at 03:00 (system time).
///
/// # Arguments
/// * `db_connection` - The MySQL database connection pool to execute the deletion.
///
/// # Returns
/// A result containing the `tokio_cron_scheduler::Job` or an `anyhow::Error`.
pub fn clean_up_outbox_job(
    db_connection: sea_orm::DatabaseConnection,
) -> Result<Job, anyhow::Error> {
    // Run daily at 03:00: "0 0 3 * * *"
    let job = Job::new_async("0 0 3 * * *", move |_uuid, _l| {
        let db_clone = db_connection.clone();
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
