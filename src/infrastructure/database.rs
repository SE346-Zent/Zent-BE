use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use migration::{MigratorTrait, Migrator};

use crate::core::config::AppConfig;

/// Mask the sensitive password component of a database connection URL for secure logging.
///
/// # Arguments
/// * `connection_url` - The full database connection string (e.g., mysql://user:pass@host/db).
///
/// # Returns
/// A string with the password replaced by '***'.
fn mask_db_url(connection_url: &str) -> String {
    // mysql://user:password@host:port/db → mysql://user:***@host:port/db
    if let Some(at_pos) = connection_url.rfind('@') {
        if let Some(colon_pos) = connection_url[..at_pos].rfind(':') {
            let prefix = &connection_url[..=colon_pos]; // "mysql://user:"
            let suffix = &connection_url[at_pos..];      // "@host:port/db"
            return format!("{}***{}", prefix, suffix);
        }
    }
    connection_url.to_string()
}

/// Initialize the primary relational database: establish connection, configure the pool, and apply migrations.
///
/// # Arguments
/// * `app_config` - Reference to the application configuration containing DB settings.
///
/// # Returns
/// A result containing the `sea_orm::DatabaseConnection` or a boxed error.
pub async fn init_database(app_config: &AppConfig) -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let masked_url = mask_db_url(&app_config.database_url);
    tracing::info!("Connecting to database: {}", masked_url);

    let mut connect_options = ConnectOptions::new(&app_config.database_url);

    connect_options.max_connections(app_config.db_max_connections)
       .min_connections(app_config.db_min_connections)
       .connect_timeout(Duration::from_secs(app_config.db_connect_timeout_seconds))
       .acquire_timeout(Duration::from_secs(app_config.db_acquire_timeout_seconds))
       .idle_timeout(Duration::from_secs(app_config.db_idle_timeout_seconds))
       .max_lifetime(Duration::from_secs(app_config.db_max_lifetime_seconds))
       .sqlx_logging(false);

    let db_connection = Database::connect(connect_options).await.map_err(|err| {
        tracing::error!(
            "Failed to connect to database at {} — error: {}",
            masked_url,
            err
        );
        Box::new(err) as Box<dyn std::error::Error>
    })?;

    tracing::info!("Running database migrations");
    Migrator::up(&db_connection, None).await.expect("Failed to run database migrations");
    tracing::info!("Database migrations applied successfully");

    Ok(db_connection)
}
