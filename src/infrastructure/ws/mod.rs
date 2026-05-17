use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// Messages exchanged between the server and clients over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WsOutgoing {
    /// Heartbeat ping — client must reply with PONG
    Ping,
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
    /// Typing indicator
    Typing {
        room_id: String,
        user_id: String,
        user_name: String,
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
    /// Heartbeat response
    Pong,
    /// Send a chat message (lightweight — just room_id + content, no binary)
    Message {
        room_id: String,
        content: Option<String>,
        image_url: Option<String>,
        reply_to: Option<String>,
    },
    /// Typing indicator
    Typing {
        room_id: String,
    },
    /// Mark messages as read
    MarkRead {
        message_ids: Vec<String>,
    },
}

/// Commands sent internally to each connection's actor loop.
#[derive(Debug, Clone)]
pub enum ConnectionCommand {
    /// Send a message to this specific client
    Send(String),
    /// Force-close the connection with a status code
    Close { code: u16, reason: String },
}

/// Manages all active WebSocket connections, keyed by user ID.
pub struct ConnectionManager {
    connections: RwLock<HashMap<Uuid, mpsc::UnboundedSender<ConnectionCommand>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new connection for a user. If the user already has a connection,
    /// the old one is closed (single-device policy — adjust if multi-device is needed).
    pub async fn register(&self, user_id: Uuid, tx: mpsc::UnboundedSender<ConnectionCommand>) {
        let mut conns = self.connections.write().await;
        if let Some(old_tx) = conns.remove(&user_id) {
            let _ = old_tx.send(ConnectionCommand::Close {
                code: 4002,
                reason: "New connection opened".to_string(),
            });
        }
        conns.insert(user_id, tx);
    }

    /// Unregister a user's connection.
    pub async fn unregister(&self, user_id: &Uuid) {
        self.connections.write().await.remove(user_id);
    }

    /// Send a message to a specific user. Returns Ok(()) if delivered,
    /// Err if the user is not connected (caller should use push notification fallback).
    pub async fn send_to_user(&self, user_id: &Uuid, message: &str) -> Result<(), ()> {
        let conns = self.connections.read().await;
        if let Some(tx) = conns.get(user_id) {
            tx.send(ConnectionCommand::Send(message.to_string()))
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
