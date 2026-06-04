use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;
use crate::infrastructure::ws::{ConnectionManager, ConnectionCommand};

/// Heartbeat: send WebSocket protocol-level Ping every 30 seconds.
/// Drops the connection after 2 missed Pong responses.
///
/// Uses `pong_received` counter (incremented by the reader task on each
/// protocol-level Pong frame) to detect whether the client is still alive.
/// Postman and other standard WebSocket clients auto-respond to Ping frames
/// with Pong frames at the protocol level — no application-level JSON needed.
///
/// Spawned per-connection from the WS handler.
pub async fn run_heartbeat(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    pong_received: Arc<AtomicU32>,
    conn_id: u64,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    let mut last_pong = pong_received.load(Ordering::Acquire);
    let mut missed = 0u32;

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown_rx.changed() => { break; }
        }

        if !manager.is_connected(&user_id).await {
            break;
        }

        // Send protocol-level Ping (Postman auto-responds with Pong)
        if tx.send(ConnectionCommand::Ping(vec![])).is_err() {
            tracing::info!(
                "[conn {}] Heartbeat: write channel closed for user {} — connection already gone, stopping heartbeat",
                conn_id, user_id
            );
            break;
        }

        // Wait 5 seconds for a Pong response (interruptible by shutdown)
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => {}
            _ = shutdown_rx.changed() => { break; }
        }

        if !manager.is_connected(&user_id).await {
            break;
        }

        let current_pong = pong_received.load(Ordering::Acquire);
        if current_pong > last_pong {
            // Pong received — reset missed counter
            missed = 0;
            last_pong = current_pong;
        } else {
            missed += 1;
            if missed >= 2 {
                tracing::warn!("Heartbeat missed 2 Pongs for user {}, dropping", user_id);
                manager.unregister(&user_id, conn_id).await;
                let _ = tx.send(ConnectionCommand::Close {
                    code: 4003,
                    reason: "Heartbeat timeout".to_string(),
                });
                break;
            }
        }
    }
}
