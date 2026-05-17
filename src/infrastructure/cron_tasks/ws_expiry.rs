use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::infrastructure::ws::{ConnectionManager, ConnectionCommand, WsOutgoing};

/// JWT expiry enforcer: warns at 5 minutes before expiry, closes at expiry.
pub async fn run_expiry_enforcer(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    ttl_seconds: i64,
) {
    if ttl_seconds > 300 {
        tokio::time::sleep(Duration::from_secs((ttl_seconds - 300) as u64)).await;
        if manager.is_connected(&user_id).await {
            let warning = serde_json::to_string(&WsOutgoing::TokenExpiring).unwrap_or_default();
            let _ = tx.send(ConnectionCommand::Send(warning));
        }
    }

    tokio::time::sleep(Duration::from_secs(300)).await;

    if manager.is_connected(&user_id).await {
        tracing::info!("Token expired for user {}, closing WebSocket", user_id);
        manager.unregister(&user_id).await;
        let _ = tx.send(ConnectionCommand::Close {
            code: 4001,
            reason: "Token expired".to_string(),
        });
    }
}
