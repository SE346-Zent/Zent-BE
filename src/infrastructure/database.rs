use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use migration::{MigratorTrait, Migrator};

use crate::core::config::AppConfig;

/// Initialize the MySQL database connection pool and run pending migrations.
///
/// Reads all connection-pool tuning parameters from `AppConfig`.
pub async fn init_database(cfg: &AppConfig) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let mut opt = ConnectOptions::new(&cfg.database_url);

    opt.max_connections(cfg.db_max_connections)
       .min_connections(cfg.db_min_connections)
       .connect_timeout(Duration::from_secs(cfg.db_connect_timeout_seconds))
       .acquire_timeout(Duration::from_secs(cfg.db_acquire_timeout_seconds))
       .idle_timeout(Duration::from_secs(cfg.db_idle_timeout_seconds))
       .max_lifetime(Duration::from_secs(cfg.db_max_lifetime_seconds))
       .sqlx_logging(false);

    let db = Database::connect(opt).await?;

    tracing::info!("Running database migrations");
    Migrator::up(&db, None).await.expect("Failed to run database migrations");
    tracing::info!("Database migrations applied successfully");

    Ok(db)
}
