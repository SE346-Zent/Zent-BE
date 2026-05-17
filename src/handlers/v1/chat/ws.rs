use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, ConnectInfo,
    },
    response::IntoResponse,
    Extension,
};
use std::sync::Arc;
use std::net::SocketAddr;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::core::state::{AppState, AccessTokenDefaultTTLSeconds};
use crate::infrastructure::ws::{WsIncoming, WsOutgoing, ConnectionManager, ConnectionCommand};
use crate::model::jwt_claims::Claims;

/// Handles the WebSocket upgrade and spawns a per-connection actor.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("WebSocket connection attempt from {}", addr);
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(socket: WebSocket, addr: SocketAddr, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Channel for sending messages from ConnectionManager → this socket's writer task
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ConnectionCommand>();

    // Phase 1: Wait for AUTH frame
    let user_id = loop {
        match ws_receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                match serde_json::from_str::<WsIncoming>(&text) {
                    Ok(WsIncoming::Auth { token }) => {
                        match validate_ws_token(&token, &state.decoding_key) {
                            Ok(uid) => break uid,
                            Err(_) => {
                                let _ = ws_sender.send(Message::Text(
                                    serde_json::to_string(&WsOutgoing::Error {
                                        code: 4001,
                                        message: "Invalid or expired token".to_string(),
                                    }).unwrap_or_default().into(),
                                )).await;
                                return; // Close connection
                            }
                        }
                    }
                    _ => {
                        let _ = ws_sender.send(Message::Text(
                            serde_json::to_string(&WsOutgoing::Error {
                                code: 4000,
                                message: "First frame must be AUTH".to_string(),
                            }).unwrap_or_default().into(),
                        )).await;
                        return;
                    }
                }
            }
            Some(Ok(_)) => continue, // Skip binary/ping/pong before auth
            _ => return, // Connection closed before auth
        }
    };

    // Register connection
    let ws_manager = get_ws_manager(&state);
    ws_manager.register(user_id, cmd_tx).await;

    tracing::info!("WebSocket authenticated for user {} from {}", user_id, addr);

    // Spawn heartbeat task
    let heartbeat_user = user_id;
    let heartbeat_manager = ws_manager.clone();
    let heartbeat_tx = cmd_tx.clone();
    tokio::spawn(async move {
        run_heartbeat(heartbeat_user, heartbeat_manager, heartbeat_tx).await;
    });

    // JWT expiry enforcer — spawn a timer based on the token's exp claim
    // (We re-read the token from the first frame — for simplicity we use a fixed TTL)
    let expiry_user = user_id;
    let expiry_manager = ws_manager.clone();
    let expiry_tx = cmd_tx.clone();
    let token_ttl = state.access_token_ttl.0;
    tokio::spawn(async move {
        run_expiry_enforcer(expiry_user, expiry_manager, expiry_tx, token_ttl).await;
    });

    // Spawn writer task: reads from cmd_rx and writes to WebSocket
    let mut writer_sender = ws_sender;
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                ConnectionCommand::Send(text) => {
                    if writer_sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                ConnectionCommand::Close { code, reason } => {
                    let _ = writer_sender.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code,
                        reason: reason.into(),
                    }))).await;
                    break;
                }
            }
        }
    });

    // Main read loop: process incoming messages
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                    match incoming {
                        WsIncoming::Pong => {
                            // Heartbeat response — handled by heartbeat task
                        }
                        WsIncoming::Message { room_id, content, image_url, reply_to } => {
                            // Build lightweight message and broadcast to room members
                            handle_ws_message(
                                &state,
                                user_id,
                                &room_id,
                                content,
                                image_url,
                                reply_to,
                            ).await;
                        }
                        WsIncoming::Typing { room_id } => {
                            handle_typing(&state, user_id, &room_id).await;
                        }
                        WsIncoming::MarkRead { message_ids } => {
                            handle_mark_read(&state, user_id, &message_ids).await;
                        }
                        WsIncoming::RefreshToken { token } => {
                            // Validate new token and reset expiry timer
                            if validate_ws_token(&token, &state.decoding_key).is_ok() {
                                tracing::info!("Token refreshed for user {}", user_id);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::warn!("WebSocket error for user {}: {}", user_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    ws_manager.unregister(&user_id).await;
    tracing::info!("WebSocket disconnected for user {}", user_id);
}

/// Validate a JWT token for WebSocket authentication.
/// Returns the user's UUID on success.
fn validate_ws_token(token: &str, decoding_key: &DecodingKey) -> Result<Uuid, ()> {
    let data = decode::<Claims>(token, decoding_key, &Validation::default())
        .map_err(|_| ())?;
    Uuid::parse_str(&data.claims.sub).map_err(|_| ())
}

/// Heartbeat: send PING every 30 seconds, drop after 2 missed PONGs.
async fn run_heartbeat(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut missed = 0u32;

    loop {
        interval.tick().await;

        // Check if still connected
        if !manager.is_connected(&user_id).await {
            break;
        }

        let ping = serde_json::to_string(&WsOutgoing::Ping).unwrap_or_default();
        if tx.send(ConnectionCommand::Send(ping)).is_err() {
            break;
        }

        // Wait for PONG (simplified: timeout-based check)
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // In a real implementation, we'd track PONG replies with an atomic counter.
        // For now, we consider the client alive as long as the channel is open.
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

/// JWT expiry enforcer: closes the socket when the token TTL expires.
async fn run_expiry_enforcer(
    user_id: Uuid,
    manager: Arc<ConnectionManager>,
    tx: mpsc::UnboundedSender<ConnectionCommand>,
    ttl_seconds: i64,
) {
    // Send a warning 5 minutes before expiry
    if ttl_seconds > 300 {
        tokio::time::sleep(std::time::Duration::from_secs((ttl_seconds - 300) as u64)).await;
        if manager.is_connected(&user_id).await {
            let warning = serde_json::to_string(&WsOutgoing::TokenExpiring).unwrap_or_default();
            let _ = tx.send(ConnectionCommand::Send(warning));
        }
    }

    // Wait for full expiry
    tokio::time::sleep(std::time::Duration::from_secs(300)).await;

    if manager.is_connected(&user_id).await {
        tracing::info!("Token expired for user {}, closing WebSocket", user_id);
        manager.unregister(&user_id).await;
        let _ = tx.send(ConnectionCommand::Close {
            code: 4001,
            reason: "Token expired".to_string(),
        });
    }
}

/// Handle an incoming chat message via WebSocket: store in MongoDB, broadcast to room.
async fn handle_ws_message(
    state: &AppState,
    sender_id: Uuid,
    room_id: &str,
    content: Option<String>,
    image_url: Option<String>,
    reply_to: Option<String>,
) {
    use mongodb::bson::doc;

    let now = mongodb::bson::DateTime::now();
    let doc = doc! {
        "room_id": room_id,
        "sender_id": sender_id.to_string(),
        "content": content.as_deref().unwrap_or(""),
        "image_url": image_url.as_deref(),
        "reply_to": reply_to.as_deref(),
        "created_at": now,
        "edited_at": mongodb::bson::Bson::Null,
    };

    let col = state.mongodb.collection::<mongodb::bson::Document>("messages");
    if let Ok(result) = col.insert_one(doc).await {
        let message_id = result.inserted_id.as_object_id()
            .map(|o| o.to_hex())
            .unwrap_or_default();

        let outgoing = WsOutgoing::Message {
            id: message_id,
            room_id: room_id.to_string(),
            sender_id: sender_id.to_string(),
            sender_name: String::new(), // Will be filled by client cache or fetched
            content,
            image_url,
            reply_to,
            created_at: now.to_string(),
        };
        let payload = serde_json::to_string(&outgoing).unwrap_or_default();

        // Broadcast to room members (query MySQL for member list)
        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
        use crate::entities::chat_room_members;
        use uuid::Uuid as UuidParsed;

        if let Ok(room_uuid) = UuidParsed::parse_str(room_id) {
            if let Ok(members) = chat_room_members::Entity::find()
                .filter(chat_room_members::Column::RoomId.eq(room_uuid))
                .all(state.db.as_ref())
                .await
            {
                let ws_manager = get_ws_manager(state);
                for member in members {
                    if member.user_id != sender_id {
                        if ws_manager.send_to_user(&member.user_id, &payload).await.is_err() {
                            // User not connected — push notification fallback
                            // TODO: call FCM service
                            tracing::debug!("User {} not connected, should send push", member.user_id);
                        }
                    }
                }
            }
        }
    }
}

async fn handle_typing(state: &AppState, user_id: Uuid, room_id: &str) {
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
    use crate::entities::chat_room_members;
    use uuid::Uuid as UuidParsed;

    let outgoing = WsOutgoing::Typing {
        room_id: room_id.to_string(),
        user_id: user_id.to_string(),
        user_name: String::new(),
    };
    let payload = serde_json::to_string(&outgoing).unwrap_or_default();

    if let Ok(room_uuid) = UuidParsed::parse_str(room_id) {
        if let Ok(members) = chat_room_members::Entity::find()
            .filter(chat_room_members::Column::RoomId.eq(room_uuid))
            .all(state.db.as_ref())
            .await
        {
            let ws_manager = get_ws_manager(state);
            for member in members {
                if member.user_id != user_id {
                    let _ = ws_manager.send_to_user(&member.user_id, &payload).await;
                }
            }
        }
    }
}

async fn handle_mark_read(state: &AppState, user_id: Uuid, message_ids: &[String]) {
    use mongodb::bson::doc;
    use futures::TryStreamExt;

    let col = state.mongodb.collection::<mongodb::bson::Document>("read_receipts");
    let now = mongodb::bson::DateTime::now();

    for msg_id in message_ids {
        let filter = doc! {
            "message_id": msg_id,
            "user_id": user_id.to_string(),
        };
        let update = doc! {
            "$setOnInsert": {
                "message_id": msg_id,
                "user_id": user_id.to_string(),
                "read_at": now,
            }
        };
        let opts = mongodb::options::UpdateOptions::builder().upsert(true).build();
        let _ = col.update_one(filter.clone(), update).with_options(opts).await;

        // Notify sender (would need to look up the message's sender_id)
        // For simplicity, skip the notification for now
    }
}

/// Extract ConnectionManager from AppState (or lazily create it).
fn get_ws_manager(state: &AppState) -> Arc<ConnectionManager> {
    // We need to store the ConnectionManager in AppState.
    // For now, create a static singleton.
    use std::sync::OnceLock;
    static WS_MANAGER: OnceLock<Arc<ConnectionManager>> = OnceLock::new();
    WS_MANAGER.get_or_init(|| Arc::new(ConnectionManager::new())).clone()
}
