use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tokio::sync::Mutex;
use std::sync::Arc;
use migration::{MigratorTrait, Migrator};

use crate::core::config::AppConfig;

pub struct DatabaseManager {
    opt: ConnectOptions,
    connection: Mutex<Option<DatabaseConnection>>,
    is_stub: bool,
}

impl DatabaseManager {
    pub async fn new(cfg: &AppConfig) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let mut opt = ConnectOptions::new(&cfg.database_url);

        opt.max_connections(cfg.db_max_connections)
           .min_connections(cfg.db_min_connections)
           .connect_timeout(Duration::from_secs(cfg.db_connect_timeout_seconds))
           .acquire_timeout(Duration::from_secs(cfg.db_acquire_timeout_seconds))
           .idle_timeout(Duration::from_secs(cfg.db_idle_timeout_seconds))
           .max_lifetime(Duration::from_secs(cfg.db_max_lifetime_seconds))
           .sqlx_logging(false);

        let manager = Arc::new(Self {
            opt,
            connection: Mutex::new(None),
            is_stub: false,
        });

        // Test initial connection
        let _ = manager.get_connection().await;

        Ok(manager)
    }

    pub async fn get_connection(&self) -> Result<DatabaseConnection, sea_orm::DbErr> {
        if self.is_stub {
            let guard = self.connection.lock().await;
            if let Some(conn) = &*guard {
                return Ok(conn.clone());
            }
            return Err(sea_orm::DbErr::Custom("Database is in stub mode without connection".to_string()));
        }

        let mut guard = self.connection.lock().await;

        if let Some(conn) = &*guard {
            if conn.ping().await.is_ok() {
                return Ok(conn.clone());
            }
        }

        let db = Database::connect(self.opt.clone()).await?;

        tracing::info!("Running database migrations");
        Migrator::up(&db, None).await.expect("Failed to run database migrations");
        tracing::info!("Database migrations applied successfully");

        *guard = Some(db.clone());
        Ok(db)
    }

    pub fn from_connection(db: DatabaseConnection) -> Arc<Self> {
        Arc::new(Self {
            opt: ConnectOptions::new("sqlite::memory:"),
            connection: Mutex::new(Some(db)),
            is_stub: true, // Mark as stub because it uses injected connection
        })
    }

    pub fn stub() -> Arc<Self> {
        Arc::new(Self {
            opt: ConnectOptions::new("sqlite::memory:"),
            connection: Mutex::new(None),
            is_stub: true,
        })
    }
}

pub async fn init_database(cfg: &AppConfig) -> Result<Arc<DatabaseManager>, Box<dyn std::error::Error>> {
    DatabaseManager::new(cfg).await
}
