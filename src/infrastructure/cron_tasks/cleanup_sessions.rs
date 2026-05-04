use tokio_cron_scheduler::Job;
use sea_orm::*;
use chrono::Utc;
use crate::entities::sessions;
use tracing::{info, error};

/// Builds a cron job that cleans up revoked or expired sessions at 12:00 PM every day.
pub fn build_cleanup_job(db: DatabaseConnection) -> Result<Job, anyhow::Error> {
    // Cron expression for 12:00 PM every day: "0 0 12 * * *"
    let job = Job::new_async("0 0 12 * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        Box::pin(async move {
            info!("Running session cleanup job...");
            if let Err(e) = cleanup_expired_sessions(&db_clone).await {
                error!("Error cleaning up sessions: {:?}", e);
            }
        })
    })?;
    Ok(job)
}

async fn cleanup_expired_sessions(db: &DatabaseConnection) -> Result<(), DbErr> {
    let now = Utc::now();

    // Delete sessions that are either:
    // 1. Revoked (revoked_at is not null)
    // 2. Expired (expires_at < now)
    let delete_result = sessions::Entity::delete_many()
        .filter(
            Condition::any()
                .add(sessions::Column::RevokedAt.is_not_null())
                .add(sessions::Column::ExpiresAt.lt(now))
        )
        .exec(db)
        .await?;

    if delete_result.rows_affected > 0 {
        info!("Cleaned up {} revoked or expired sessions", delete_result.rows_affected);
    }

    Ok(())
}
