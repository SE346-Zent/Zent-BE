use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
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
    mut reset_rx: watch::Receiver<Instant>,
    conn_id: u64,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let ttl = Duration::from_secs(ttl_seconds as u64);

    loop {
        let deadline = *reset_rx.borrow();

        // Warn at 5 minutes before expiry (only when TTL exceeds 5 min).
        // Use checked_sub to avoid underflow when less than 300s remain.
        if ttl > Duration::from_secs(300) {
            if let Some(warn_at) = deadline.checked_sub(Duration::from_secs(300)) {
                tokio::select! {
                    _ = tokio::time::sleep_until(warn_at) => {
                        if manager.is_connected(&user_id).await {
                            let warning = serde_json::to_string(&WsOutgoing::TokenExpiring)
                                .unwrap_or_default();
                            let _ = tx.send(ConnectionCommand::Send(warning));
                        }
                    }
                    _ = reset_rx.changed() => { continue; }
                    _ = shutdown_rx.changed() => { break; }
                }
            }
            // else: less than 300s remain — skip warning, go straight to expiry
        }

        // Wait for expiry (or a reset or shutdown)
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => {
                if manager.is_connected(&user_id).await {
                    tracing::info!("Token expired for user {}, closing WebSocket", user_id);
                    manager.unregister(&user_id, conn_id).await;
                    let _ = tx.send(ConnectionCommand::Close {
                        code: 4001,
                        reason: "Token expired".to_string(),
                    });
                }
                break;
            }
            _ = reset_rx.changed() => { continue; }
            _ = shutdown_rx.changed() => { break; }
        }
    }
}
