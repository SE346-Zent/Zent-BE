use std::sync::Arc;
use redis::AsyncCommands;
use uuid::Uuid;
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::ws::ConnectionManager;

/// Cleans up stale `chat:viewing:*` Valkey keys for users who are no longer connected.
pub async fn cleanup_stale_viewing_keys(
    valkey: &Option<Arc<ValkeyClient>>,
    ws_manager: &Arc<ConnectionManager>,
) -> Result<u64, anyhow::Error> {
    let vc = match valkey {
        Some(v) => v,
        None => return Ok(0),
    };

    let mut conn = vc.get_connection().await?;

    // Use SCAN instead of KEYS to avoid blocking Valkey on large keyspaces.
    // The scan iterator borrows `conn` mutably, so scope it to release the borrow
    // before the deletion loop below.
    let keys: Vec<String> = {
        let mut keys = Vec::new();
        let mut iter = conn.scan_match("chat:viewing:*").await?;
        while let Some(key) = iter.next_item().await {
            keys.push(key?);
        }
        keys
    }; // iter dropped here — conn borrow released

    let mut cleaned = 0u64;
    for key in &keys {
        // Extract user_id from key pattern chat:viewing:{user_id}
        let uid_str = key.trim_start_matches("chat:viewing:");
        if let Ok(user_id) = Uuid::parse_str(uid_str) {
            if !ws_manager.is_connected(&user_id).await {
                let _: () = conn.del(key).await.unwrap_or_default();
                cleaned += 1;
            }
        }
    }

    if cleaned > 0 {
        tracing::info!("Cleaned {} stale chat:viewing keys", cleaned);
    }

    Ok(cleaned)
}
