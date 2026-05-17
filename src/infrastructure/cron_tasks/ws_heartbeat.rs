use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::infrastructure::ws::{ConnectionManager, ConnectionCommand, WsOutgoing};

/// Heartbeat: send PING every 30 seconds, drop after 2 missed PONGs.
/// Spawned per-connection from the WS handler.
pub async fn run_heartbeat(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut missed = 0u32;

    loop {
        interval.tick().await;

        if !manager.is_connected(&user_id).await {
            break;
        }

        let ping = serde_json::to_string(&WsOutgoing::Ping).unwrap_or_default();
        if tx.send(ConnectionCommand::Send(ping)).is_err() {
            break;
        }

        tokio::time::sleep(Duration::from_secs(5)).await;

        if !manager.is_connected(&user_id).await {
            break;
        }
        missed += 1;
        if missed >= 2 {
            tracing::warn!("Heartbeat missed 2 PONGs for user {}, dropping", user_id);
            manager.unregister(&user_id).await;
            let _ = tx.send(ConnectionCommand::Close {
                code: 4003,
                reason: "Heartbeat timeout".to_string(),
            });
            break;
        }
    }
}
