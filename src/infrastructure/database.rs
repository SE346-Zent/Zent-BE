use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use std::sync::Arc;
use migration::{MigratorTrait, Migrator};

use crate::core::config::AppConfig;

/// Thin wrapper around SeaORM's `DatabaseConnection`.
/// SeaORM already manages connection pooling internally via sqlx,
/// so this struct simply holds the connection without additional
/// reconnection or mutex logic.
pub struct DatabasePool {
    connection: Option<DatabaseConnection>,
}

impl DatabasePool {
    /// Returns a clone of the underlying `DatabaseConnection`.
    /// SeaORM connections are cheaply cloneable (they share the pool).
    pub async fn get_connection(&self) -> Result<DatabaseConnection, sea_orm::DbErr> {
        self.connection.clone().ok_or_else(|| {
            sea_orm::DbErr::Custom("Database is in stub mode without connection".to_string())
        })
    }

    /// Wrap an existing `DatabaseConnection` (used in integration tests
    /// with in-memory SQLite).
    pub fn from_connection(db: DatabaseConnection) -> Arc<Self> {
        Arc::new(Self {
            connection: Some(db),
        })
    }

    /// Create a non-functional stub for tests that don't need DB access.
    pub fn stub() -> Arc<Self> {
        Arc::new(Self {
            connection: None,
        })
    }
}

/// Backward-compatible alias so existing tests that reference
/// `DatabaseManager` continue to compile.
pub type DatabaseManager = DatabasePool;

/// Initialize database: connect, configure pool, run migrations.
pub async fn init_database(cfg: &AppConfig) -> Result<Arc<DatabasePool>, Box<dyn std::error::Error>> {
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

    Ok(Arc::new(DatabasePool {
        connection: Some(db),
    }))
}
