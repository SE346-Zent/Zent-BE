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
    /// Authenticate with JWT. `session_id` links this WS to a login session.
    Auth { token: String, session_id: Option<String> },
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

/// Tracks a single connection entry with its channel sender, a monotonic ID
/// used to prevent stale cleanup, and the session ID linking the WS to a login session.
struct ConnectionEntry {
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    conn_id: u64,
    session_id: Uuid,
}

/// Manages all active WebSocket connections, keyed by user ID.
/// Each user may have multiple concurrent connections (one per device/session).
pub struct ConnectionManager {
    connections: RwLock<HashMap<Uuid, Vec<ConnectionEntry>>>,
    next_conn_id: std::sync::atomic::AtomicU64,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            next_conn_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Register a new connection for a user under a specific session.
    ///
    /// If the same `session_id` already has a connection (e.g. client refreshed
    /// the WebSocket), the old entry is replaced and its socket is closed.
    /// Connections from other sessions are preserved (multi-device support).
    ///
    /// Returns a unique connection ID for this registration.
    pub async fn register(&self, user_id: Uuid, session_id: Uuid, tx: mpsc::UnboundedSender<ConnectionCommand>) -> u64 {
        let conn_id = self.next_conn_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut conns = self.connections.write().await;
        let entries = conns.entry(user_id).or_insert_with(Vec::new);

        // If this session already has a connection, close and replace it
        if let Some(pos) = entries.iter().position(|e| e.session_id == session_id) {
            let old = entries.remove(pos);
            tracing::info!(
                "[user {}] WebSocket REPLACING old connection (conn {}, session {}) with new connection (conn {}) — closing old socket with code 4002",
                user_id, old.conn_id, session_id, conn_id
            );
            let _ = old.tx.send(ConnectionCommand::Close {
                code: 4002,
                reason: "New connection opened for same session".to_string(),
            });
        } else {
            tracing::info!(
                "[user {}] WebSocket NEW connection (conn {}, session {}) — total connections: {}",
                user_id, conn_id, session_id, entries.len() + 1
            );
        }

        entries.push(ConnectionEntry { tx, conn_id, session_id });
        conn_id
    }

    /// Unregister a specific connection by `conn_id`.
    /// Only removes the entry if `conn_id` matches, preventing stale cleanup
    /// from interfering with newer connections.
    pub async fn unregister(&self, user_id: &Uuid, conn_id: u64) {
        let mut conns = self.connections.write().await;
        if let Some(entries) = conns.get_mut(user_id) {
            if let Some(pos) = entries.iter().position(|e| e.conn_id == conn_id) {
                entries.remove(pos);
            }
            if entries.is_empty() {
                conns.remove(user_id);
            }
        }
    }

    /// Check if a user has at least one active connection.
    pub async fn is_connected(&self, user_id: &Uuid) -> bool {
        self.connections.read().await.get(user_id).map_or(false, |e| !e.is_empty())
    }

    /// Send a message to ALL connections of a user (broadcast to all devices).
    /// Returns Ok(()) if at least one delivery succeeded, Err if user has no connections.
    pub async fn send_to_user(&self, user_id: &Uuid, message: &str) -> Result<(), ()> {
        let conns = self.connections.read().await;
        if let Some(entries) = conns.get(user_id) {
            if entries.is_empty() {
                return Err(());
            }
            for entry in entries {
                let _ = entry.tx.send(ConnectionCommand::Send(message.to_string()));
            }
            Ok(())
        } else {
            Err(())
        }
    }

    /// Send a message to a specific session's connection only.
    /// Returns Ok(()) if the session was found and the message was queued.
    pub async fn send_to_session(&self, user_id: &Uuid, session_id: &Uuid, message: &str) -> Result<(), ()> {
        let conns = self.connections.read().await;
        if let Some(entries) = conns.get(user_id) {
            if let Some(entry) = entries.iter().find(|e| e.session_id == *session_id) {
                return entry.tx.send(ConnectionCommand::Send(message.to_string())).map_err(|_| ());
            }
        }
        Err(())
    }

    /// Close all WebSocket connections for a specific session (used when a session is revoked).
    /// Sends code 4004 ("Session revoked") to each matching connection.
    pub async fn close_session_connections(&self, user_id: &Uuid, session_id: &Uuid) {
        let conns = self.connections.read().await;
        if let Some(entries) = conns.get(user_id) {
            for entry in entries.iter().filter(|e| e.session_id == *session_id) {
                tracing::info!(
                    "[conn {}] Closing WebSocket for user {} session {} — session revoked (code 4004)",
                    entry.conn_id, user_id, session_id
                );
                let _ = entry.tx.send(ConnectionCommand::Close {
                    code: 4004,
                    reason: "Session revoked".to_string(),
                });
            }
        }
    }

    /// Return the number of active connections for a user.
    pub async fn user_connection_count(&self, user_id: &Uuid) -> usize {
        self.connections.read().await.get(user_id).map_or(0, |e| e.len())
    }

    /// Return all session IDs that have active connections for a user.
    pub async fn get_user_session_ids(&self, user_id: &Uuid) -> Vec<Uuid> {
        self.connections.read().await
            .get(user_id)
            .map(|entries| entries.iter().map(|e| e.session_id).collect())
            .unwrap_or_default()
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
