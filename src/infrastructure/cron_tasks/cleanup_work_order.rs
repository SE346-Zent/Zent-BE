use tokio_cron_scheduler::Job;
use sea_orm::*;
use std::sync::Arc;
use tracing::{info, error};

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;

pub fn clean_up_work_order_job(
    db: DatabaseConnection,
    luts: Arc<LookupTables>,
    valkey: Option<Arc<ValkeyClient>>,
    rabbitmq: Option<Arc<lapin::Connection>>,
) -> Result<Job, anyhow::Error> {
    // Run every 1 hour at the top of the hour: "0 0 * * * *"
    let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
        let db_clone = db.clone();
        let luts_clone = luts.clone();
        let valkey_clone = valkey.clone();
        let rabbitmq_clone = rabbitmq.clone();
        Box::pin(async move {
            info!("Running unassigned work order cleanup job...");
            if let Err(e) = crate::handlers::v1::work_orders::run_cleanup(
                &db_clone,
                &luts_clone,
                valkey_clone,
                &rabbitmq_clone,
            )
            .await
            {
                error!("Error in cleanup job: {:?}", e);
            }
        })
    })?;
    Ok(job)
}
