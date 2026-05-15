use std::collections::HashMap;
use std::path::Path;
use redis::{Client, aio::MultiplexedConnection, RedisError};
use tokio::fs;
use tracing::{warn, info};
use crate::core::config::AppConfig;

/// Read a Lua script from the configured scripts directory.
async fn read_lua_script(base_dir: &str, filename: &str) -> Result<String, RedisError> {
    let path = Path::new(base_dir).join(filename);
    fs::read_to_string(&path)
        .await
        .map_err(|e| RedisError::from((
            redis::ErrorKind::Io,
            "Failed to read Lua script",
            format!("Path: {}, Error: {}", path.display(), e)
        )))
}

/// Thin wrapper around a redis `Client`.
///
/// Instead of caching a single `MultiplexedConnection` (which can suffer from
/// "broken pipe" errors when the server restarts or the connection times out),
/// we store the `Client` and create a fresh `MultiplexedConnection` on every
/// call. `Client::get_multiplexed_async_connection` is a cheap operation that
/// creates a new connection pool handle; the underlying TCP connections are
/// managed internally by the redis crate.
///
/// Lua script SHA hashes are loaded once at startup and stored immutably.
pub struct ValkeyClient {
    client: Client,
    script_hashes: HashMap<String, String>,
}

impl ValkeyClient {
    /// Creates a fresh `MultiplexedConnection` from the underlying `Client`.
    /// This ensures we never hand out a stale/broken connection — every
    /// call gets a new pool handle to the Valkey server.
    pub async fn get_connection(&self) -> Result<MultiplexedConnection, RedisError> {
        self.client.get_multiplexed_async_connection().await
    }

    /// Returns a copy of the pre-loaded Lua script SHA hashes.
    pub fn get_script_hashes(&self) -> HashMap<String, String> {
        self.script_hashes.clone()
    }
}

/// Initialize Valkey: open client, load Lua scripts from the filesystem, return wrapper.
pub async fn init_cache(cfg: &AppConfig) -> Result<ValkeyClient, RedisError> {
    let db_index = match cfg.app_stage.as_str() {
        "production" => 0,
        _ => 1,
    };

    let base_url = cfg.valkey_url.trim_end_matches('/');
    let connection_url = format!("{}/{}", base_url, db_index);

    let client = Client::open(connection_url.as_str())?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    // Determine the Lua scripts directory with a fallback mechanism
    let mut lua_dir = cfg.lua_script_dir.clone();
    if !Path::new(&lua_dir).is_dir() {
        let fallback = "src/infrastructure/lua_script";
        if Path::new(fallback).is_dir() {
            warn!("Configured LUA_SCRIPT_DIR '{}' not found, falling back to '{}'", lua_dir, fallback);
            lua_dir = fallback.to_string();
        }
    }
    info!("Loading Lua scripts from: {}", lua_dir);

    let mut script_hashes = HashMap::new();

    let verify_otp_lua = read_lua_script(&lua_dir, "verify_otp.lua").await?;
    let sha: String = redis::cmd("SCRIPT")
        .arg("LOAD")
        .arg(verify_otp_lua)
        .query_async(&mut conn)
        .await?;
    script_hashes.insert("verify_otp".to_string(), sha);

    let check_idempotency_lua = read_lua_script(&lua_dir, "check_idempotency.lua").await?;
    let sha: String = redis::cmd("SCRIPT")
        .arg("LOAD")
        .arg(check_idempotency_lua)
        .query_async(&mut conn)
        .await?;
    script_hashes.insert("check_idempotency".to_string(), sha);

    // Drop the initial connection — a fresh one is created on every get_connection() call
    drop(conn);

    info!("Valkey cache initialized (db {}, {} Lua scripts loaded)", db_index, script_hashes.len());

    Ok(ValkeyClient { client, script_hashes })
}
