use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use sea_orm::*;
use chrono::{Utc, Duration};
use crate::entities::users;
use crate::core::lookup_tables::LookupTables;
use tracing::{info, error};

pub async fn start_scheduler(db: DatabaseConnection, lookup_tables: Arc<LookupTables>) -> Result<(), anyhow::Error> {
    let sched = JobScheduler::new().await?;

    // Create a job that runs every 10 minutes to cleanup pending accounts
    // Cron expression: "0 1/10 * * * *" (every 10 minutes)
    let db_clone = db.clone();
    let lut_clone = lookup_tables.clone();
    let job = Job::new_async("0 1/10 * * * *", move |_uuid, _l| {
        let db = db_clone.clone();
        let lut = lut_clone.clone();
        Box::pin(async move {
            info!("Running pending account cleanup job...");
            if let Err(e) = cleanup_pending_accounts(&db, &lut).await {
                error!("Error cleaning up pending accounts: {:?}", e);
            }
        })
    })?;

    sched.add(job).await?;
    sched.start().await?;

    info!("Scheduler started successfully");
    Ok(())
}

async fn cleanup_pending_accounts(db: &DatabaseConnection, lut: &LookupTables) -> Result<(), DbErr> {
    // 1. Get the "Pending" status ID from memory
    if let Some(&pending_id) = lut.account_statuses_by_name.get("Pending") {
        let one_hour_ago = Utc::now() - Duration::hours(1);

        // 2. Delete users with "Pending" status created more than 1 hour ago
        let delete_result = users::Entity::delete_many()
            .filter(users::Column::AccountStatus.eq(pending_id))
            .filter(users::Column::CreatedAt.lt(one_hour_ago))
            .exec(db)
            .await?;

        if delete_result.rows_affected > 0 {
            info!("Cleaned up {} expired pending accounts", delete_result.rows_affected);
        }
    } else {
        error!("'Pending' account status not found in memory lookup tables");
    }

    Ok(())
}
