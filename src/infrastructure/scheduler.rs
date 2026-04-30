use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

pub struct AppScheduler {
    pub sched: JobScheduler,
}

impl AppScheduler {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let sched = JobScheduler::new().await?;
        Ok(Self { sched })
    }

    pub async fn register_job(&self, job: Job) -> Result<(), anyhow::Error> {
        self.sched.add(job).await?;
        Ok(())
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        self.sched.start().await?;
        info!("Scheduler started successfully");
        Ok(())
    }
}