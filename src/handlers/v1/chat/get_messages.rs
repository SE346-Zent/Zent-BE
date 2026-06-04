use axum::{extract::{State, Path, Query}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use mongodb::{bson::{doc, oid::ObjectId}, options::FindOptions, Database as MongoDatabase};
use futures::TryStreamExt;
use serde::Deserialize;
use uuid::Uuid;
use redis::AsyncCommands;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::chat::message_response::MessageResponse;
use crate::entities::{users, chat_room_members};

#[derive(Deserialize, Debug, utoipa::IntoParams)]
pub struct GetMessagesQuery {
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 { 50 }

#[utoipa::path(
    get, path = "/api/v1/chat/rooms/{id}/messages",
    params(
        ("id" = String, Path, description = "The unique identifier of the chat room"),
        GetMessagesQuery
    ),
    responses(
        (status = 200, description = "Paginated message history", body = ApiResponse<Vec<MessageResponse>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Room not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    tag = "chat",
    security(("bearer_auth" = []))
)]
pub async fn get_messages(
    Extension(_auth): Extension<AuthUser>,
    State(mongo): State<Arc<MongoDatabase>>,
    State(_db): State<Arc<DatabaseConnection>>,
    State(valkey): State<Option<Arc<ValkeyClient>>>,
    Path(room_id): Path<String>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<ApiResponse<Vec<MessageResponse>>>, AppError> {
    let user_id = _auth.user.id;

    // Verify the authenticated user is a member of this room
    let room_uuid = Uuid::parse_str(&room_id)
        .map_err(|_| AppError::BadRequest("Invalid room ID".to_string()))?;
    let is_member = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::RoomId.eq(room_uuid))
        .filter(chat_room_members::Column::UserId.eq(user_id))
        .one(_db.as_ref())
        .await?
        .is_some();
    if !is_member {
        return Err(AppError::Forbidden("You do not have access to this chat room".to_string()));
    }

    // Reset unread count to 0 in Valkey when user opens this chat
    if let Some(ref vc) = valkey {
        if let Ok(mut conn) = vc.get_connection().await {
            let unread_key = format!("chat:unread:{}:{}", room_id, user_id);
            let _: () = conn.set(&unread_key, 0u64).await.unwrap_or_default();
            let _: () = conn.expire(&unread_key, 86400).await.unwrap_or_default();
        }
    }

    let mut filter = doc! { "room_id": &room_id };
    if let Some(ref cursor) = query.cursor {
        if let Ok(oid) = ObjectId::parse_str(cursor) {
            filter.insert("_id", doc! { "$lt": oid });
        }
    }

    let opts = FindOptions::builder()
        .sort(doc! { "_id": -1 })
        .limit(query.limit)
        .build();

    let col = mongo.collection::<mongodb::bson::Document>("messages");
    let mut cursor = col.find(filter).with_options(opts).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MongoDB error: {}", e)))?;

    let mut messages: Vec<mongodb::bson::Document> = Vec::new();
    while let Some(doc) = cursor.try_next().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("MongoDB cursor error: {}", e)))? {
        messages.push(doc);
    }

    let sender_ids: Vec<String> = messages.iter()
        .filter_map(|m| m.get_str("sender_id").ok())
        .map(|s| s.to_string())
        .collect();

    let sender_map = if sender_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        let sender_uuids: Vec<Uuid> = sender_ids.iter()
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect();
        users::Entity::find()
            .filter(users::Column::Id.is_in(sender_uuids))
            .all(_db.as_ref())
            .await?
            .into_iter()
            .map(|u| (u.id.to_string(), u.full_name))
            .collect::<std::collections::HashMap<_, _>>()
    };

    let data: Vec<MessageResponse> = messages
        .into_iter()
        .map(|m| {
            let id = m.get_object_id("_id").map(|o| o.to_hex()).unwrap_or_default();
            let sender_id = m.get_str("sender_id").unwrap_or("").to_string();
            let sender_name = sender_map.get(&sender_id).cloned().unwrap_or_else(|| "Unknown".to_string());
            crate::services::v1::chat::map_message_response::map_to_message_response(
                id,
                m.get_str("room_id").unwrap_or("").to_string(),
                sender_id,
                sender_name,
                m.get_str("content").ok().map(|s| s.to_string()),
                m.get_str("image_url").ok().map(|s| s.to_string()),
                m.get_str("reply_to").ok().map(|s| s.to_string()),
                m.get_datetime("created_at").map(|d| d.to_string()).unwrap_or_default(),
                m.get_datetime("edited_at").ok().map(|d| d.to_string()),
                vec![],
            )
        })
        .collect();

    Ok(Json(ApiResponse::success(200, "Messages retrieved successfully", data)))
}
