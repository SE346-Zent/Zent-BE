use axum::{extract::{State, Query}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, QuerySelect};
use uuid::Uuid;
use mongodb::{bson::doc, Database as MongoDatabase};
use redis::AsyncCommands;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::chat::list_rooms_query::ListRoomsQuery;
use crate::model::requests::pagination::PaginationRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::responses::chat::room_response::ChatRoomResponse;
use crate::entities::{chat_rooms, chat_room_members, users};

#[utoipa::path(
    get, path = "/api/v1/chat/rooms", params(ListRoomsQuery),
    responses(
        (status = 200, description = "List of 1-on-1 chat rooms", body = ApiResponse<Vec<ChatRoomResponse>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    tag = "chat",
    security(("bearer_auth" = []))
)]
pub async fn list_rooms(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(mongo): State<Arc<MongoDatabase>>,
    State(valkey): State<Option<Arc<ValkeyClient>>>,
    Query(query): Query<ListRoomsQuery>,
) -> Result<Json<ApiResponse<Vec<ChatRoomResponse>>>, AppError> {
    let PaginationRequest { page, limit } = query.pagination;
    let user_id = auth.user.id;

    let my_memberships = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::UserId.eq(user_id))
        .all(db.as_ref())
        .await?;

    if my_memberships.is_empty() {
        let pagination = PaginationResponse::new(limit, page, 0);
        return Ok(Json(ApiResponse::success_with_meta(200, "Chat rooms retrieved successfully", vec![], pagination)));
    }

    let room_ids: Vec<Uuid> = my_memberships.iter().map(|m| m.room_id).collect();

    let all_members = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::RoomId.is_in(room_ids.clone()))
        .all(db.as_ref())
        .await?;

    let mut room_opponents: std::collections::HashMap<Uuid, Uuid> = std::collections::HashMap::new();
    for member in &all_members {
        if member.user_id != user_id {
            room_opponents.insert(member.room_id, member.user_id);
        }
    }

    let opponent_ids: Vec<Uuid> = room_opponents.values().cloned().collect();
    let opponent_users: std::collections::HashMap<Uuid, users::Model> = if opponent_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        users::Entity::find()
            .filter(users::Column::Id.is_in(opponent_ids))
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect()
    };

    let rooms = chat_rooms::Entity::find()
        .filter(chat_rooms::Column::Id.is_in(room_ids.clone()))
        .order_by_desc(chat_rooms::Column::CreatedAt)
        .offset((page - 1) * limit)
        .limit(limit)
        .all(db.as_ref())
        .await?;

    let total_records = chat_rooms::Entity::find()
        .filter(chat_rooms::Column::Id.is_in(room_ids))
        .count(db.as_ref())
        .await?;

    // Open Valkey connection once for batch unread reads
    let mut valkey_conn = if let Some(ref vc) = valkey {
        vc.get_connection().await.ok()
    } else {
        None
    };

    let mut data: Vec<ChatRoomResponse> = Vec::new();
    for room in &rooms {
        let opponent_id = room_opponents.get(&room.id);
        let opponent = opponent_id.and_then(|oid| opponent_users.get(oid));

        let opposite_name = opponent.map(|u| u.full_name.clone()).unwrap_or_else(|| "Unknown".to_string());
        let opposite_avatar = opponent.and_then(|u| u.avatar_url.clone());

        // Latest message from MongoDB
        let msg_col = mongo.collection::<mongodb::bson::Document>("messages");
        let latest_msg = msg_col
            .find_one(doc! { "room_id": room.id.to_string() })
            .sort(doc! { "_id": -1 })
            .await
            .ok()
            .flatten();

        let latest_message = latest_msg.as_ref()
            .and_then(|m| m.get_str("content").ok())
            .map(|s| s.to_string());
        let latest_message_at = latest_msg.as_ref()
            .and_then(|m| m.get_datetime("created_at").ok())
            .map(|d| d.to_string());

        // Unread count from Valkey
        let unread_key = format!("chat:unread:{}:{}", room.id, user_id);
        let unread_count: u64 = if let Some(ref mut conn) = valkey_conn {
            conn.get(&unread_key).await.unwrap_or(0u64)
        } else {
            0
        };

        data.push(crate::services::v1::chat::map_room_response::map_to_room_response(
            room.id,
            opposite_name,
            opposite_avatar,
            latest_message,
            latest_message_at,
            unread_count,
        ));
    }

    let pagination = PaginationResponse::new(limit, page, total_records);
    Ok(Json(ApiResponse::success_with_meta(200, "Chat rooms retrieved successfully", data, pagination)))
}
