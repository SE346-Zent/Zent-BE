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
    // 1. Get necessary IDs from memory lookup tables
    let pending_status_id = lut.account_statuses_by_name.get("Pending").copied();
    let customer_role_id = lut.roles_by_name.get("Customer").copied();
    let admin_role_id = lut.roles_by_name.get("Admin").copied();
    let superadmin_role_id = lut.roles_by_name.get("SuperAdmin").copied();
    let technician_role_id = lut.roles_by_name.get("Technician").copied();

    if let Some(pending_id) = pending_status_id {
        // Fetch cleanup times from policies or use defaults
        let customer_cleanup_hours = lut.policies.get("pending_customer_cleanup_hours")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(1);
        let staff_cleanup_days = lut.policies.get("pending_staff_cleanup_days")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(3);

        // For testing: use 1 minute. For production, uncomment the lines below.
        let _ = customer_cleanup_hours;
        let _ = staff_cleanup_days;
        // let customer_cutoff = Utc::now() - Duration::minutes(1);
        // let staff_cutoff = Utc::now() - Duration::minutes(1);

        let customer_cutoff = Utc::now() - Duration::hours(customer_cleanup_hours);
        let staff_cutoff = Utc::now() - Duration::days(staff_cleanup_days);

        // 2. Build the condition for deletion
        let mut cleanup_condition = Condition::any();

        // Condition A: Pending Customer created > X hours ago
        if let Some(customer_id) = customer_role_id {
            cleanup_condition = cleanup_condition.add(
                Condition::all()
                    .add(users::Column::RoleId.eq(customer_id))
                    .add(users::Column::CreatedAt.lt(customer_cutoff))
            );
        }

        // Condition B: Pending staff (Admin, SuperAdmin, Technician) created > Y days ago
        let mut staff_role_ids = Vec::new();
        if let Some(id) = admin_role_id { staff_role_ids.push(id); }
        if let Some(id) = superadmin_role_id { staff_role_ids.push(id); }
        if let Some(id) = technician_role_id { staff_role_ids.push(id); }

        if !staff_role_ids.is_empty() {
            cleanup_condition = cleanup_condition.add(
                Condition::all()
                    .add(users::Column::RoleId.is_in(staff_role_ids))
                    .add(users::Column::CreatedAt.lt(staff_cutoff))
            );
        }

        // 3. Find matching users
        let expired_users = users::Entity::find()
            .filter(users::Column::AccountStatus.eq(pending_id))
            .filter(cleanup_condition)
            .limit(100)
            .all(db)
            .await?;

        let mut deleted_count = 0;
        for user in expired_users {
            match users::Entity::delete_by_id(user.id).exec(db).await {
                Ok(_) => {
                    deleted_count += 1;
                }
                Err(e) => {
                    // Log the error but continue with other users
                    error!("Failed to delete expired pending user {} (Role: {}): {:?}", user.id, user.role_id, e);
                }
            }
        }

        if deleted_count > 0 {
            info!("Cleaned up {} expired pending accounts", deleted_count);
        }
    } else {
        error!("'Pending' account status not found in memory lookup tables");
    }

    Ok(())
}
