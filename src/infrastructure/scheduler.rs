use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use sea_orm::*;
use chrono::{Utc, Duration};
use crate::entities::{users, account_status};
use tracing::{info, error};

pub async fn start_scheduler(db: DatabaseConnection) -> Result<(), anyhow::Error> {
    let sched = JobScheduler::new().await?;

    // Create a job that runs every 10 minutes to cleanup pending accounts
    // Cron expression: "0 1/10 * * * *" (every 10 minutes)
    let db_clone = db.clone();
    let job = Job::new_async("0 1/10 * * * *", move |_uuid, _l| {
        let db = db_clone.clone();
        Box::pin(async move {
            info!("Running pending account cleanup job...");
            if let Err(e) = cleanup_pending_accounts(&db).await {
                error!("Error cleaning up pending accounts: {:?}", e);
            }
        })
    })?;

    sched.add(job).await?;
    sched.start().await?;

    info!("Scheduler started successfully");
    Ok(())
}

async fn cleanup_pending_accounts(db: &DatabaseConnection) -> Result<(), DbErr> {
    // 1. Find the "Pending" status ID
    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(db)
        .await?;

    if let Some(status) = pending_status {
        let one_hour_ago = Utc::now() - Duration::hours(1);

        // 2. Delete users with "Pending" status created more than 1 hour ago
        let delete_result = users::Entity::delete_many()
            .filter(users::Column::AccountStatus.eq(status.id))
            .filter(users::Column::CreatedAt.lt(one_hour_ago))
            .exec(db)
            .await?;

        if delete_result.rows_affected > 0 {
            info!("Cleaned up {} expired pending accounts", delete_result.rows_affected);
        }
    }

    Ok(())
}
