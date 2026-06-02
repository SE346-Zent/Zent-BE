use std::sync::Arc;
use tokio_cron_scheduler::Job;
use sea_orm::*;
use chrono::Utc;
use crate::entities::warranties;
use crate::core::lookup_tables::LookupTables;
use tracing::{info, error};

/// Builds a cron job that expires warranties whose end_date has passed.
/// Runs at 00:00 every day.
pub fn build_expire_warranties_job(db: DatabaseConnection, lookup_tables: Arc<LookupTables>) -> Result<Job, anyhow::Error> {
    let job = Job::new_async("0 0 0 * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let lut_clone = lookup_tables.clone();
        Box::pin(async move {
            info!("Running warranty expiration job...");
            if let Err(e) = expire_warranties(&db_clone, &lut_clone).await {
                error!("Error expiring warranties: {:?}", e);
            }
        })
    })?;
    Ok(job)
}

async fn expire_warranties(db: &DatabaseConnection, lut: &LookupTables) -> Result<(), DbErr> {
    let now = Utc::now();

    let expired_status_id = match lut.warranty_statuses_by_name.get("Expired") {
        Some(id) => *id,
        None => {
            error!("'Expired' warranty status not found in lookup tables");
            return Ok(());
        }
    };

    let active_status = "Active".to_string();
    let active_id = lut.warranty_statuses_by_name.get("Active").copied();

    // Find warranties that are still marked Active but have passed their end_date
    let mut condition = Condition::all()
        .add(warranties::Column::EndDate.lt(now))
        .add(warranties::Column::DeletedAt.is_null());

    // Match by status name or status id
    let mut status_condition = Condition::any()
        .add(warranties::Column::WarrantyStatus.eq(&active_status));
    if let Some(active_id) = active_id {
        status_condition = status_condition.add(warranties::Column::WarrantyStatusId.eq(active_id));
    }
    condition = condition.add(status_condition);

    let expired_warranties = warranties::Entity::find()
        .filter(condition)
        .all(db)
        .await?;

    let count = expired_warranties.len();
    for warranty in expired_warranties {
        let mut active_model: warranties::ActiveModel = warranty.into();
        active_model.warranty_status = Set("Expired".to_string());
        active_model.warranty_status_id = Set(Some(expired_status_id));
        active_model.updated_at = Set(Utc::now());
        active_model.update(db).await?;
    }

    if count > 0 {
        info!("Expired {} warranties", count);
    }

    Ok(())
}
