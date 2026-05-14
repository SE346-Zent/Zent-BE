use redis::{Client, aio::MultiplexedConnection, RedisError};
use std::collections::HashMap;
use tracing::warn;
use crate::core::config::AppConfig;

/// Atomic OTP verification script loaded at compile time.
pub const VERIFY_OTP_LUA: &str = include_str!("lua_script/verify_otp.lua");

/// Atomic idempotency check script loaded at compile time.
pub const CHECK_IDEMPOTENCY_LUA: &str = include_str!("lua_script/check_idempotency.lua");

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

/// Initialize Valkey: open client, load Lua scripts, return wrapper.
pub async fn init_cache(cfg: &AppConfig) -> Result<ValkeyClient, RedisError> {
    let db_index = match cfg.app_stage.as_str() {
        "production" => 0,
        _ => 1,
    };

    let base_url = cfg.valkey_url.trim_end_matches('/');
    let connection_url = format!("{}/{}", base_url, db_index);

    let client = Client::open(connection_url.as_str())?;
    let mut conn = client.get_multiplexed_async_connection().await?;

    // Pre-load Lua scripts (stored server-side; we only keep the SHA hashes)
    let mut script_hashes = HashMap::new();

    let verify_otp_sha: String = redis::cmd("SCRIPT")
        .arg("LOAD")
        .arg(VERIFY_OTP_LUA)
        .query_async(&mut conn)
        .await?;

    script_hashes.insert("verify_otp".to_string(), verify_otp_sha);

    let check_idempotency_sha: String = redis::cmd("SCRIPT")
        .arg("LOAD")
        .arg(CHECK_IDEMPOTENCY_LUA)
        .query_async(&mut conn)
        .await?;

    script_hashes.insert("check_idempotency".to_string(), check_idempotency_sha);

    // Drop the initial connection — a fresh one is created on every get_connection() call
    drop(conn);

    warn!("Valkey cache initialized (db {}) — fresh connections will be created per-request", db_index);

    Ok(ValkeyClient { client, script_hashes })
}
