use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State, ConnectInfo,
    },
    response::IntoResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::Instant;
use uuid::Uuid;
use jsonwebtoken::{decode, DecodingKey, Validation};
use redis::AsyncCommands;
use crate::core::state::AppState;
use crate::infrastructure::ws::{WsIncoming, WsOutgoing, ConnectionCommand, ShutdownSignal};
use crate::infrastructure::metrics;

const MAX_WS_FRAME_BYTES: usize = 1 * 1024 * 1024; // 1 MiB
use crate::infrastructure::cron_tasks::{ws_heartbeat, ws_expiry};
use crate::model::jwt_claims::Claims;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("WebSocket connection attempt from {}", addr);
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(socket: WebSocket, addr: SocketAddr, state: AppState) {
    // Local connection ID used for tracing the whole connection lifetime.
    // Distinct from `user_conn_id` (assigned by the manager) so we can also
    // trace pre-auth activity.
    let trace_id = Uuid::new_v4();

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<ConnectionCommand>();

    // Phase 1: Wait for AUTH frame (with timeout)
    let auth_deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let user_id = loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if text.len() > MAX_WS_FRAME_BYTES {
                            tracing::warn!(
                                "[trace {}] WebSocket auth FAILED from {} — first frame is {} bytes, exceeds limit of {} bytes (code 4013)",
                                trace_id, addr, text.len(), MAX_WS_FRAME_BYTES
                            );
                            let _ = ws_sender.send(Message::Text(
                                serde_json::to_string(&WsOutgoing::Error {
                                    code: 4013,
                                    message: "Frame too large".to_string(),
                                }).unwrap_or_default().into(),
                            )).await;
                            metrics::init().ws_auth_fail_total.add(1, &[opentelemetry::KeyValue::new("reason", "frame_too_large")]);
                            return;
                        }
                        match serde_json::from_str::<WsIncoming>(&text) {
                            Ok(WsIncoming::Auth { token }) => {
                                match validate_ws_token(&token, &state.decoding_key) {
                                    Ok(uid) => break uid,
                                    Err(reason) => {
                                        tracing::warn!(
                                            "[trace {}] WebSocket auth FAILED from {} — reason: {} (code 4001)",
                                            trace_id, addr, reason
                                        );
                                        let _ = ws_sender.send(Message::Text(
                                            serde_json::to_string(&WsOutgoing::Error {
                                                code: 4001,
                                                message: "Invalid or expired token".to_string(),
                                            }).unwrap_or_default().into(),
                                        )).await;
                                        metrics::init().ws_auth_fail_total.add(1, &[opentelemetry::KeyValue::new("reason", "invalid_token")]);
                                        return;
                                    }
                                }
                            }
                            Ok(other) => {
                                tracing::warn!(
                                    "[trace {}] WebSocket auth FAILED from {} — first frame was not AUTH, got: {:?} (code 4000)",
                                    trace_id, addr, other
                                );
                                let _ = ws_sender.send(Message::Text(
                                    serde_json::to_string(&WsOutgoing::Error {
                                        code: 4000,
                                        message: "First frame must be AUTH".to_string(),
                                    }).unwrap_or_default().into(),
                                )).await;
                                metrics::init().ws_auth_fail_total.add(1, &[opentelemetry::KeyValue::new("reason", "not_auth_frame")]);
                                return;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "[trace {}] WebSocket auth FAILED from {} — first frame is not valid JSON: {} (code 4002)",
                                    trace_id, addr, e
                                );
                                let _ = ws_sender.send(Message::Text(
                                    serde_json::to_string(&WsOutgoing::Error {
                                        code: 4002,
                                        message: "Invalid JSON in first frame".to_string(),
                                    }).unwrap_or_default().into(),
                                )).await;
                                metrics::init().ws_auth_fail_total.add(1, &[opentelemetry::KeyValue::new("reason", "invalid_json")]);
                                return;
                            }
                        }
                    }
                    Some(Ok(Message::Close(close_frame))) => {
                        tracing::info!(
                            "[trace {}] WebSocket client at {} closed connection before AUTH (close frame: {:?})",
                            trace_id, addr, close_frame
                        );
                        return;
                    }
                    Some(Ok(other)) => {
                        // Non-text, non-Close frames (Binary, Ping, Pong) before AUTH —
                        // log at debug level since clients may probe with Pings early.
                        tracing::debug!(
                            "[trace {}] WebSocket from {} received non-AUTH frame during auth (frame: {:?}), waiting for AUTH",
                            trace_id, addr, other
                        );
                        continue;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(
                            "[trace {}] WebSocket transport ERROR during auth from {}: {} — connection will be closed",
                            trace_id, addr, e
                        );
                        return;
                    }
                    None => {
                        tracing::warn!(
                            "[trace {}] WebSocket client at {} DISCONNECTED before sending AUTH frame (stream ended) — possible client bug, network drop, or proxy timeout",
                            trace_id, addr
                        );
                        return;
                    }
                }
            }
            _ = tokio::time::sleep_until(auth_deadline) => {
                tracing::warn!(
                    "[trace {}] WebSocket auth TIMEOUT from {} — client connected but never sent AUTH within 30s. \
                     This usually means: (1) client bug, (2) proxy/NAT keeping the TCP socket open, \
                     or (3) client did not receive the WebSocket upgrade response. Closing with code 4008.",
                    trace_id, addr
                );
                let _ = ws_sender.send(Message::Text(
                    serde_json::to_string(&WsOutgoing::Error {
                        code: 4008,
                        message: "Authentication timeout".to_string(),
                    }).unwrap_or_default().into(),
                )).await;
                let _ = ws_sender.send(Message::Close(None)).await;
                return;
            }
        }
    };

    let ws_manager = crate::infrastructure::ws::get_ws_manager();
    let user_conn_id = ws_manager.register(user_id, cmd_tx.clone()).await;
    tracing::info!("WebSocket authenticated for user {} (conn {}) from {}", user_id, user_conn_id, addr);

    // Track active connection
    metrics::init().ws_connections_active.add(1, &[]);

    // Heartbeat pong counter — incremented by reader on each protocol-level Pong
    let pong_counter = Arc::new(AtomicU32::new(0));

    // Deadline channel so RefreshToken can extend the connection lifetime
    let initial_deadline = Instant::now() + Duration::from_secs(state.access_token_ttl.0 as u64);
    let (deadline_tx, deadline_rx) = tokio::sync::watch::channel(initial_deadline);

    // Cooperative shutdown signal — old-connection tasks stop when a new
    // connection registers for the same user.
    let shutdown = ShutdownSignal::new();

    // Spawn heartbeat + expiry from cron_tasks
    let hb_user = user_id;
    let hb_mgr = ws_manager.clone();
    let hb_tx = cmd_tx.clone();
    let hb_pong = pong_counter.clone();
    let hb_shutdown = shutdown.receiver();
    tokio::spawn(async move { ws_heartbeat::run_heartbeat(hb_user, hb_mgr, hb_tx, hb_pong, user_conn_id, hb_shutdown).await });

    let ex_user = user_id;
    let ex_mgr = ws_manager.clone();
    let ex_tx = cmd_tx.clone();
    let ttl = state.access_token_ttl.0;
    let ex_shutdown = shutdown.receiver();
    tokio::spawn(async move { ws_expiry::run_expiry_enforcer(ex_user, ex_mgr, ex_tx, ttl, deadline_rx, user_conn_id, ex_shutdown).await });

    // Writer task
    let writer_user_id = user_id;
    let writer_conn_id = user_conn_id;
    let mut writer_sender = ws_sender;
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                ConnectionCommand::Send(text) => {
                    if writer_sender.send(Message::Text(text.into())).await.is_err() {
                        tracing::warn!(
                            "[conn {}] Writer task: failed to send text message to user {} — client socket is broken, exiting writer loop",
                            writer_conn_id, writer_user_id
                        );
                        break;
                    }
                }
                ConnectionCommand::Ping(data) => {
                    if writer_sender.send(Message::Ping(data.into())).await.is_err() {
                        tracing::info!(
                            "[conn {}] Writer task: failed to send Ping to user {} — socket broken, exiting writer loop",
                            writer_conn_id, writer_user_id
                        );
                        break;
                    }
                }
                ConnectionCommand::Close { code, reason } => {
                    tracing::info!(
                        "[conn {}] Writer task: sending Close frame (code={}, reason={:?}) to user {}",
                        writer_conn_id, code, reason, writer_user_id
                    );
                    let _ = writer_sender.send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code,
                        reason: reason.into(),
                    }))).await;
                    break;
                }
            }
        }
        tracing::debug!("[conn {}] Writer task exited for user {}", writer_conn_id, writer_user_id);
    });

    // Main read loop. `had_explicit_break` distinguishes between
    // a loop that exited via `break` (Close frame or transport error)
    // versus a loop that ended because `next()` returned `None`
    // (client disconnected without sending a Close frame).
    let mut had_explicit_break = false;
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if text.len() > MAX_WS_FRAME_BYTES {
                    tracing::warn!("Oversized frame ({} bytes) from user {}", text.len(), user_id);
                    let _ = cmd_tx.send(ConnectionCommand::Send(
                        serde_json::to_string(&WsOutgoing::Error {
                            code: 4013,
                            message: "Frame too large".to_string(),
                        }).unwrap_or_default(),
                    ));
                    break;
                }
                if let Ok(incoming) = serde_json::from_str::<WsIncoming>(&text) {
                    match incoming {
                        WsIncoming::Viewing { room_id } => {
                            handle_viewing(&state, user_id, &room_id).await;
                        }
                        WsIncoming::Leaving => {
                            handle_leaving(&state, user_id).await;
                        }
                        WsIncoming::Message { room_id, content, image_url, reply_to } => {
                            handle_ws_message(&state, user_id, &room_id, content, image_url, reply_to).await;
                        }
                        WsIncoming::MarkRead { message_ids } => {
                            handle_mark_read(&state, user_id, &message_ids).await;
                        }
                        WsIncoming::RefreshToken { token } => {
                            if validate_ws_token(&token, &state.decoding_key).is_ok() {
                                tracing::info!("Token refreshed for user {}", user_id);
                                let new_deadline = Instant::now() + Duration::from_secs(state.access_token_ttl.0 as u64);
                                let _ = deadline_tx.send(new_deadline);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Ping(_data)) => {
                // Tungstenite auto-responds to protocol-level Pings with Pongs.
                // No application-level handling needed.
            }
            Ok(Message::Pong(_)) => {
                // Protocol-level Pong from client — increment heartbeat counter
                pong_counter.fetch_add(1, Ordering::Release);
            }
            Ok(Message::Close(close_frame)) => {
                tracing::info!(
                    "[conn {}] WebSocket closed by client for user {} — close frame: {:?}",
                    user_conn_id, user_id, close_frame
                );
                had_explicit_break = true;
                break;
            }
            Err(e) => {
                tracing::warn!(
                    "[conn {}] WebSocket TRANSPORT ERROR for user {}: {} — dropping connection",
                    user_conn_id, user_id, e
                );
                had_explicit_break = true;
                break;
            }
            _ => {}
        }
    }

    // Detect whether the read loop exited because the client sent a Close frame,
    // a transport error happened, or the client just disappeared (no Close frame).
    // We infer "no Close frame" by checking the last entry into the loop after
    // the explicit `break`s above — if we get here, the stream ended.
    if !had_explicit_break {
        tracing::warn!(
            "[conn {}] WebSocket stream ended WITHOUT a Close frame for user {} — \
             client likely disconnected abruptly (network drop, app killed, NAT timeout, \
             or proxy closed the connection). No graceful close handshake received.",
            user_conn_id, user_id
        );
    }

    // Cleanup — stop spawned tasks and conditionally unregister.
    // The conn_id check prevents a stale connection's cleanup from
    // unregistering a newer connection for the same user.
    shutdown.shutdown();
    let still_us = ws_manager.user_conn_id(&user_id).await == Some(user_conn_id);
    if still_us {
        handle_leaving(&state, user_id).await;
    }
    ws_manager.unregister(&user_id, user_conn_id).await;

    // Track connection close
    metrics::init().ws_connections_active.add(-1, &[]);
    let close_reason = if had_explicit_break { "explicit_close" } else { "stream_ended" };
    metrics::init().ws_closed_total.add(1, &[
        opentelemetry::KeyValue::new("reason", close_reason),
    ]);

    tracing::info!(
        "[conn {}] WebSocket disconnected for user {} (cleanup complete, viewing_state_cleared={})",
        user_conn_id, user_id, still_us
    );
}

fn validate_ws_token(token: &str, decoding_key: &DecodingKey) -> Result<Uuid, &'static str> {
    use jsonwebtoken::errors::ErrorKind;
    let data = decode::<Claims>(token, decoding_key, &Validation::default())
        .map_err(|e| match e.kind() {
            ErrorKind::ExpiredSignature => "token expired",
            ErrorKind::InvalidSignature => "invalid signature",
            ErrorKind::InvalidToken => "malformed token",
            ErrorKind::InvalidSubject => "invalid 'sub' claim",
            ErrorKind::InvalidIssuer => "invalid 'iss' claim",
            ErrorKind::InvalidAudience => "invalid 'aud' claim",
            ErrorKind::ImmatureSignature => "token not yet valid (nbf)",
            ErrorKind::MissingRequiredClaim(_) => "missing required claim",
            ErrorKind::Base64(_) => "base64 decode error in token",
            ErrorKind::Json(_) => "JSON error in token payload",
            ErrorKind::Utf8(_) => "UTF-8 error in token payload",
            _ => "invalid token",
        })?;
    Uuid::parse_str(&data.claims.sub).map_err(|_| "subject 'sub' is not a valid UUID")
}

/// Mark user as viewing a room: set chat:viewing:{uid} and reset unread.
async fn handle_viewing(state: &AppState, user_id: Uuid, room_id: &str) {
    if let Some(ref vc) = state.valkey {
        if let Ok(mut conn) = vc.get_connection().await {
            let viewing_key = format!("chat:viewing:{}", user_id);
            let unread_key = format!("chat:unread:{}:{}", room_id, user_id);
            let _: () = conn.set(&viewing_key, room_id).await.unwrap_or_default();
            let _: () = conn.set(&unread_key, 0u64).await.unwrap_or_default();
            let _: () = conn.expire(&unread_key, 86400).await.unwrap_or_default();
        }
    }
}

/// Clear viewing state when user leaves a chat.
async fn handle_leaving(state: &AppState, user_id: Uuid) {
    if let Some(ref vc) = state.valkey {
        if let Ok(mut conn) = vc.get_connection().await {
            let viewing_key = format!("chat:viewing:{}", user_id);
            let _: () = conn.del(&viewing_key).await.unwrap_or_default();
        }
    }
}

/// Store message in MongoDB, broadcast to room, increment unread for non-viewing recipients.
async fn handle_ws_message(
    state: &AppState,
    sender_id: Uuid,
    room_id: &str,
    content: Option<String>,
    image_url: Option<String>,
    reply_to: Option<String>,
) {
    use mongodb::bson::doc;
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
    use crate::entities::{chat_room_members, users};
    use uuid::Uuid as UuidParsed;

    // Look up the sender's display name for the notification and WS payload
    let sender_name = users::Entity::find_by_id(sender_id)
        .one(state.db.as_ref())
        .await
        .ok()
        .flatten()
        .map(|u| u.full_name)
        .unwrap_or_else(|| "Unknown".to_string());

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
    let result = match col.insert_one(doc).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                sender_id = %sender_id,
                room_id = room_id,
                error = %e,
                "Failed to insert chat message into MongoDB — message lost"
            );
            return;
        }
    };

    let message_id = result.inserted_id.as_object_id()
        .map(|o| o.to_hex())
        .unwrap_or_default();

    // Clone values that will also be needed for push notifications
    let content_preview = content.clone();
    let msg_id_for_notif = message_id.clone();
    let sender_name_for_notif = sender_name.clone();

    let outgoing = WsOutgoing::Message {
        id: message_id,
        room_id: room_id.to_string(),
        sender_id: sender_id.to_string(),
        sender_name,
        content,
        image_url,
        reply_to,
        created_at: now.to_string(),
    };
    let payload = serde_json::to_string(&outgoing).unwrap_or_default();

    // Get room members and deliver
    if let Ok(room_uuid) = UuidParsed::parse_str(room_id) {
        if let Ok(members) = chat_room_members::Entity::find()
            .filter(chat_room_members::Column::RoomId.eq(room_uuid))
            .all(state.db.as_ref())
            .await
        {
            let ws_manager = crate::infrastructure::ws::get_ws_manager();

            for member in members {
                if member.user_id == sender_id {
                    continue;
                }

                // Check if recipient is viewing this room
                let is_viewing = if let Some(ref vc) = state.valkey {
                    if let Ok(mut conn) = vc.get_connection().await {
                        let viewing_key = format!("chat:viewing:{}", member.user_id);
                        let viewing_room: Option<String> = conn.get(&viewing_key).await.unwrap_or(None);
                        viewing_room.as_deref() == Some(room_id)
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Try WebSocket delivery
                let send_result = ws_manager.send_to_user(&member.user_id, &payload).await;
                if send_result.is_ok() {
                    metrics::init().ws_messages_sent_total.add(1, &[
                        opentelemetry::KeyValue::new("type", "chat_message"),
                    ]);
                }

                if !is_viewing {
                    // Recipient is not viewing — increment unread + send push notification
                    if let Some(ref vc) = state.valkey {
                        if let Ok(mut conn) = vc.get_connection().await {
                            let unread_key = format!("chat:unread:{}:{}", room_id, member.user_id);
                            let _: () = conn.incr(&unread_key, 1).await.unwrap_or_default();
                            let _: () = conn.expire(&unread_key, 86400).await.unwrap_or_default();
                        }
                    }

                    // Send push notification via FCM
                    let notification_data = serde_json::json!({
                        "room_id": room_id,
                        "message_id": msg_id_for_notif,
                        "sender_id": sender_id.to_string(),
                        "sender_name": &sender_name_for_notif,
                    });
                    let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                        state.mongodb.as_ref(),
                        state.valkey.clone(),
                        state.db.as_ref(),
                        member.user_id,
                        "chat_message",
                        &format!("New message from {}", sender_name_for_notif),
                        content_preview.as_deref().unwrap_or("Sent an attachment"),
                        notification_data,
                    ).await;
                }
                // else: is_viewing → don't increment unread, don't send push notification
            }
        }
    }
}

async fn handle_mark_read(state: &AppState, user_id: Uuid, message_ids: &[String]) {
    use mongodb::bson::doc;

    if message_ids.is_empty() {
        return;
    }

    let col = state.mongodb.collection::<mongodb::bson::Document>("read_receipts");
    let now = mongodb::bson::DateTime::now();
    let user_id_str = user_id.to_string();

    // Execute upserts concurrently (up to 10 at a time) instead of sequentially
    let mut tasks = tokio::task::JoinSet::new();
    for msg_id in message_ids {
        let col = col.clone();
        let msg_id = msg_id.clone();
        let user_id_str = user_id_str.clone();
        let filter = doc! {
            "message_id": &msg_id,
            "user_id": &user_id_str,
        };
        let update = doc! {
            "$setOnInsert": {
                "message_id": &msg_id,
                "user_id": &user_id_str,
                "read_at": now,
            }
        };
        tasks.spawn(async move {
            let _ = col.update_one(filter, update)
                .upsert(true)
                .await;
        });
    }

    while let Some(_result) = tasks.join_next().await {}
}

