use redis::Client;

use crate::core::config::AppConfig;

/// Initialize the Valkey (Redis-compatible) connection client.
///
/// Database (sheet) selection:
/// - **Sheet 0** → Production stage
/// - **Sheet 1** → Local / development stage
///
/// The `APP_STAGE` environment variable controls which database is used.
/// If `APP_STAGE` is `"production"`, sheet 0 is selected; otherwise sheet 1.
pub fn init_cache(cfg: &AppConfig) -> Result<Client, redis::RedisError> {
    let db_index = match cfg.app_stage.as_str() {
        "production" => 0,
        _ => 1, // local, development, staging, etc.
    };

    // Build the connection URL with the selected database index
    // e.g. "redis://localhost:6379/1"
    let base_url = cfg.valkey_url.trim_end_matches('/');
    let connection_url = format!("{}/{}", base_url, db_index);

    tracing::info!(
        stage = %cfg.app_stage,
        db_index = db_index,
        cache_url = connection_url,
        "Initializing Valkey cache connection"
    );

    let client = Client::open(connection_url.as_str())?;

    tracing::info!("Valkey cache client initialized successfully");

    Ok(client)
}
