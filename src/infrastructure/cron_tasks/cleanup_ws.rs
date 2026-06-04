use std::sync::Arc;
use redis::AsyncCommands;
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::ws::ConnectionManager;

/// Cleans up stale `chat:viewing:*` Valkey keys for sessions that are no longer connected.
/// Viewing keys are now keyed by session_id (chat:viewing:{session_id}) and have a 24h TTL,
/// so this cleanup is a safety net for edge cases where the TTL hasn't fired yet.
pub async fn cleanup_stale_viewing_keys(
    valkey: &Option<Arc<ValkeyClient>>,
    _ws_manager: &Arc<ConnectionManager>,
) -> Result<u64, anyhow::Error> {
    let vc = match valkey {
        Some(v) => v,
        None => return Ok(0),
    };

    let mut conn = vc.get_connection().await?;

    // Use SCAN instead of KEYS to avoid blocking Valkey on large keyspaces.
    let keys: Vec<String> = {
        let mut keys = Vec::new();
        let mut iter = conn.scan_match("chat:viewing:*").await?;
        while let Some(key) = iter.next_item().await {
            keys.push(key?);
        }
        keys
    };

    // Viewing keys now have a 24h TTL set at creation time.
    // This cleanup is a safety net — we rely on TTL for normal expiry.
    // Only clean up keys that have been around longer than expected (via TTL check).
    let mut cleaned = 0u64;
    for key in &keys {
        let ttl: i64 = conn.ttl(key).await.unwrap_or(-1);
        // TTL of -1 means no expiry was set (legacy keys or edge case) — clean those up
        if ttl == -1 {
            let _: () = conn.del(key).await.unwrap_or_default();
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        tracing::info!("Cleaned {} stale chat:viewing keys (no TTL)", cleaned);
    }

    Ok(cleaned)
}
