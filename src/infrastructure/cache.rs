use redis::{Client, aio::MultiplexedConnection};
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::config::AppConfig;

/// Atomic OTP verification script loaded at compile time.
pub const VERIFY_OTP_LUA: &str = include_str!("lua_script/verify_otp.lua");

/// Thin wrapper around the redis `Client`.
/// The redis crate's `MultiplexedConnection` handles multiplexing and
/// internal reconnection, so we cache a single connection created at
/// init time and clone it on each request (cloning is cheap).
/// Lua script hashes are loaded once at startup and stored immutably.
pub struct ValkeyClient {
    connection: Option<MultiplexedConnection>,
    script_hashes: HashMap<String, String>,
}

impl ValkeyClient {
    /// Returns a clone of the cached multiplexed connection.
    /// `MultiplexedConnection` is cheaply cloneable and handles
    /// internal reconnection automatically.
    pub async fn get_connection(&self) -> Result<MultiplexedConnection, redis::RedisError> {
        self.connection.clone().ok_or_else(|| {
            redis::RedisError::from((redis::ErrorKind::InvalidClientConfig, "Valkey is in stub mode"))
        })
    }

    /// Returns a copy of the pre-loaded Lua script SHA hashes.
    pub async fn get_script_hashes(&self) -> HashMap<String, String> {
        self.script_hashes.clone()
    }

    /// Create a non-functional stub for tests that don't need cache access.
    pub fn stub() -> Arc<Self> {
        Arc::new(Self {
            connection: None,
            script_hashes: HashMap::new(),
        })
    }
}

/// Backward-compatible alias so existing tests that reference
/// `ValkeyManager` continue to compile.
pub type ValkeyManager = ValkeyClient;

/// Initialize Valkey: connect, load Lua scripts, return client.
pub async fn init_cache(cfg: &AppConfig) -> Result<Arc<ValkeyClient>, redis::RedisError> {
    let db_index = match cfg.app_stage.as_str() {
        "production" => 0,
        _ => 1,
    };

    let base_url = cfg.valkey_url.trim_end_matches('/');
    let connection_url = format!("{}/{}", base_url, db_index);

    let client = Client::open(connection_url.as_str())?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    // Pre-load Lua scripts
    let mut script_hashes = HashMap::new();

    let verify_otp_sha: String = redis::cmd("SCRIPT")
        .arg("LOAD")
        .arg(VERIFY_OTP_LUA)
        .query_async(&mut conn)
        .await?;

    script_hashes.insert("verify_otp".to_string(), verify_otp_sha);

    Ok(Arc::new(ValkeyClient {
        connection: Some(conn),
        script_hashes,
    }))
}
