use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use uuid::Uuid;
use crate::infrastructure::ws::{ConnectionManager, ConnectionCommand, WsOutgoing};

/// JWT expiry enforcer: warns at 5 minutes before expiry, closes at expiry.
///
/// Listens on `reset_rx` for deadline extensions pushed by the `RefreshToken`
/// WebSocket message. When a new token arrives, the handler sends a fresh
/// deadline through this channel and the enforcer restarts its timers
/// accordingly — so the connection lifetime is effectively extended.
pub async fn run_expiry_enforcer(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    ttl_seconds: i64,
    mut reset_rx: tokio::sync::watch::Receiver<Instant>,
) {
    let ttl = Duration::from_secs(ttl_seconds as u64);

    loop {
        let deadline = *reset_rx.borrow();

        // Warn at 5 minutes before expiry (only when TTL exceeds 5 min)
        if ttl > Duration::from_secs(300) {
            let warn_at = deadline - Duration::from_secs(300);
            tokio::select! {
                _ = tokio::time::sleep_until(warn_at) => {
                    if manager.is_connected(&user_id).await {
                        let warning = serde_json::to_string(&WsOutgoing::TokenExpiring)
                            .unwrap_or_default();
                        let _ = tx.send(ConnectionCommand::Send(warning));
                    }
                }
                _ = reset_rx.changed() => { continue; }
            }
        }

        // Wait for expiry (or a reset)
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                if manager.is_connected(&user_id).await {
                    tracing::info!("Token expired for user {}, closing WebSocket", user_id);
                    manager.unregister(&user_id).await;
                    let _ = tx.send(ConnectionCommand::Close {
                        code: 4001,
                        reason: "Token expired".to_string(),
                    });
                }
                break;
            }
            _ = reset_rx.changed() => { continue; }
        }
    }
}
