use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use migration::{MigratorTrait, Migrator};

use crate::core::config::AppConfig;

/// Mask the password portion of a MySQL connection URL for safe logging.
fn mask_db_url(url: &str) -> String {
    // mysql://user:password@host:port/db → mysql://user:***@host:port/db
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let prefix = &url[..=colon_pos]; // "mysql://user:"
            let suffix = &url[at_pos..];      // "@host:port/db"
            return format!("{}***{}", prefix, suffix);
        }
    }
    url.to_string()
}

/// Initialize database: connect, configure pool, run migrations.
pub async fn init_database(cfg: &AppConfig) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let masked = mask_db_url(&cfg.database_url);
    tracing::info!("Connecting to database: {}", masked);

    let mut opt = ConnectOptions::new(&cfg.database_url);

    opt.max_connections(cfg.db_max_connections)
       .min_connections(cfg.db_min_connections)
       .connect_timeout(Duration::from_secs(cfg.db_connect_timeout_seconds))
       .acquire_timeout(Duration::from_secs(cfg.db_acquire_timeout_seconds))
       .idle_timeout(Duration::from_secs(cfg.db_idle_timeout_seconds))
       .max_lifetime(Duration::from_secs(cfg.db_max_lifetime_seconds))
       .sqlx_logging(false);

    let db = Database::connect(opt).await.map_err(|e| {
        tracing::error!(
            "Failed to connect to database at {} — error: {}",
            masked,
            e
        );
        Box::new(e) as Box<dyn std::error::Error>
    })?;

    tracing::info!("Running database migrations");
    Migrator::up(&db, None).await.expect("Failed to run database migrations");
    tracing::info!("Database migrations applied successfully");

    Ok(db)
}
