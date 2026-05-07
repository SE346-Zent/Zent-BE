use tokio_cron_scheduler::Job;
use sea_orm::*;
use tracing::{info, error};

use crate::core::lookup_tables::LookupTables;

pub fn build_auto_assign_job(
    db: DatabaseConnection,
    luts: std::sync::Arc<LookupTables>,
    rabbitmq: Option<std::sync::Arc<lapin::Connection>>,
    templates: std::sync::Arc<std::collections::HashMap<String, String>>,
) -> Result<Job, anyhow::Error> {
    // Run every 1 hour at the top of the hour: "0 0 * * * *"
    let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let luts_clone = luts.clone();
        let rabbitmq_clone = rabbitmq.clone();
        let templates_clone = templates.clone();
        Box::pin(async move {
            info!("Running auto-assign job...");
            if let Err(e) = crate::handlers::v1::work_orders::schedule(
                &db_clone,
                &luts_clone,
                &rabbitmq_clone,
                &templates_clone,
            )
            .await
            {
                error!("Error in auto-assign job: {:?}", e);
            }
        })
    })?;
    Ok(job)
}
