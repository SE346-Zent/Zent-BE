use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Messages exchanged between the server and clients over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WsOutgoing {
    /// A new chat message delivered in real-time
    Message {
        id: String,
        room_id: String,
        sender_id: String,
        sender_name: String,
        content: Option<String>,
        image_url: Option<String>,
        reply_to: Option<String>,
        created_at: String,
    },
    /// Read receipt notification
    ReadReceipt {
        message_id: String,
        user_id: String,
        read_at: String,
    },
    /// Error message
    Error {
        code: u16,
        message: String,
    },
    /// Token expired — client should refresh
    TokenExpiring,
}

/// Incoming messages from the client.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WsIncoming {
    /// Authenticate with JWT
    Auth { token: String },
    /// Refresh token in-band
    RefreshToken { token: String },
    /// Send a chat message (lightweight — just room_id + content, no binary)
    Message {
        room_id: String,
        content: Option<String>,
        image_url: Option<String>,
        reply_to: Option<String>,
    },
    /// User is viewing a chat room (resets unread)
    Viewing {
        room_id: String,
    },
    /// User left a chat room
    Leaving,
    /// Mark messages as read
    MarkRead {
        message_ids: Vec<String>,
    },
}

/// Commands sent internally to each connection's actor loop.
#[derive(Debug, Clone)]
pub enum ConnectionCommand {
    /// Send a text message to this specific client
    Send(String),
    /// Send a WebSocket protocol-level Ping frame
    Ping(Vec<u8>),
    /// Force-close the connection with a status code
    Close { code: u16, reason: String },
}

/// Cooperative shutdown signal for spawned connection tasks (heartbeat, expiry).
/// Backed by a `tokio::sync::watch` channel — calling `shutdown()` sets the
/// signal to `true` and receivers detect it via `tokio::select!`.
pub struct ShutdownSignal {
    tx: tokio::sync::watch::Sender<bool>,
    rx: tokio::sync::watch::Receiver<bool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::watch::channel(false);
        Self { tx, rx }
    }

    /// Returns a receiver that becomes ready when `shutdown()` is called.
    pub fn receiver(&self) -> tokio::sync::watch::Receiver<bool> {
        self.rx.clone()
    }

    /// Signal all receivers to shut down.
    pub fn shutdown(&self) {
        let _ = self.tx.send(true);
    }
}

/// Tracks a single connection entry with its channel sender and a monotonic ID
/// used to prevent stale cleanup from interfering with newer connections.
struct ConnectionEntry {
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    conn_id: u64,
}

/// Manages all active WebSocket connections, keyed by user ID.
pub struct ConnectionManager {
    connections: RwLock<HashMap<Uuid, ConnectionEntry>>,
    next_conn_id: std::sync::atomic::AtomicU64,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            next_conn_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Register a new connection for a user. If the user already has a connection,
    /// the old one is closed (single-device policy — adjust if multi-device is needed).
    ///
    /// Returns a unique connection ID that callers should pass to `unregister()` so
    /// stale cleanup from replaced connections is safely ignored.
    pub async fn register(&self, user_id: Uuid, tx: mpsc::UnboundedSender<ConnectionCommand>) -> u64 {
        let conn_id = self.next_conn_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut conns = self.connections.write().await;
        if let Some(old_entry) = conns.remove(&user_id) {
            let _ = old_entry.tx.send(ConnectionCommand::Close {
                code: 4002,
                reason: "New connection opened".to_string(),
            });
        }
        conns.insert(user_id, ConnectionEntry { tx, conn_id });
        conn_id
    }

    /// Unregister a user's connection, but only if `conn_id` still matches the
    /// current entry. This prevents a replaced connection's cleanup from
    /// unregistering a newer connection for the same user.
    pub async fn unregister(&self, user_id: &Uuid, conn_id: u64) {
        let mut conns = self.connections.write().await;
        if let Some(entry) = conns.get(user_id) {
            if entry.conn_id == conn_id {
                conns.remove(user_id);
            }
        }
    }

    /// Return the current connection ID for a user, if connected.
    pub async fn user_conn_id(&self, user_id: &Uuid) -> Option<u64> {
        self.connections.read().await.get(user_id).map(|e| e.conn_id)
    }

    /// Send a message to a specific user. Returns Ok(()) if delivered,
    /// Err if the user is not connected (caller should use push notification fallback).
    pub async fn send_to_user(&self, user_id: &Uuid, message: &str) -> Result<(), ()> {
        let conns = self.connections.read().await;
        if let Some(entry) = conns.get(user_id) {
            entry.tx.send(ConnectionCommand::Send(message.to_string()))
                .map_err(|_| ())
        } else {
            Err(())
        }
    }

    /// Check if a user is currently connected.
    pub async fn is_connected(&self, user_id: &Uuid) -> bool {
        self.connections.read().await.contains_key(user_id)
    }
}

/// Return a reference to the global WebSocket ConnectionManager singleton.
///
/// The manager is lazily initialized on first access and lives for the
/// lifetime of the process. It is shared across all WebSocket connections
/// and is also used by the notification system to check whether a user
/// is currently online before deciding to send an FCM push.
pub fn get_ws_manager() -> Arc<ConnectionManager> {
    use std::sync::OnceLock;
    static WS_MANAGER: OnceLock<Arc<ConnectionManager>> = OnceLock::new();
    WS_MANAGER.get_or_init(|| Arc::new(ConnectionManager::new())).clone()
}
