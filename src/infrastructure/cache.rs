use redis::{Client, aio::MultiplexedConnection};
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::core::config::AppConfig;

/// Atomic OTP verification script loaded at compile time.
pub const VERIFY_OTP_LUA: &str = include_str!("lua_script/verify_otp.lua");

pub struct ValkeyManager {
    client: Client,
    connection: Mutex<Option<MultiplexedConnection>>,
    script_hashes: Mutex<HashMap<String, String>>,
    is_stub: bool,
}

impl ValkeyManager {
    pub async fn new(cfg: &AppConfig) -> Result<Arc<Self>, redis::RedisError> {
        let db_index = match cfg.app_stage.as_str() {
            "production" => 0,
            _ => 1,
        };

        let base_url = cfg.valkey_url.trim_end_matches('/');
        let connection_url = format!("{}/{}", base_url, db_index);

        let client = Client::open(connection_url.as_str())?;
        
        let manager = Arc::new(Self {
            client,
            connection: Mutex::new(None),
            script_hashes: Mutex::new(HashMap::new()),
            is_stub: false,
        });

        // Test initial connection
        let _ = manager.get_connection().await;

        Ok(manager)
    }

    pub async fn get_connection(&self) -> Result<MultiplexedConnection, redis::RedisError> {
        if self.is_stub {
            return Err(redis::RedisError::from((redis::ErrorKind::InvalidClientConfig, "Valkey is in stub mode")));
        }

        let mut guard = self.connection.lock().await;

        if let Some(conn) = &*guard {
            let mut conn_clone = conn.clone();
            if redis::cmd("PING").query_async::<String>(&mut conn_clone).await.is_ok() {
                return Ok(conn.clone());
            }
        }

        let conn = self.client.get_multiplexed_async_connection().await?;
        
        // Re-load Lua scripts
        let mut hashes_guard = self.script_hashes.lock().await;
        let mut conn_clone = conn.clone();
        
        let verify_otp_sha: String = redis::cmd("SCRIPT")
            .arg("LOAD")
            .arg(VERIFY_OTP_LUA)
            .query_async(&mut conn_clone)
            .await?;
        
        hashes_guard.insert("verify_otp".to_string(), verify_otp_sha);
        
        *guard = Some(conn.clone());
        Ok(conn)
    }

    pub async fn get_script_hashes(&self) -> HashMap<String, String> {
        let guard = self.script_hashes.lock().await;
        guard.clone()
    }

    pub fn stub() -> Arc<Self> {
        let client = Client::open("redis://invalid:6379").unwrap();
        Arc::new(Self {
            client,
            connection: Mutex::new(None),
            script_hashes: Mutex::new(HashMap::new()),
            is_stub: true,
        })
    }
}

pub async fn init_cache(cfg: &AppConfig) -> Result<Arc<ValkeyManager>, redis::RedisError> {
    ValkeyManager::new(cfg).await
}
