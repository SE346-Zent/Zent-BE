use axum::{extract::{State, Path, Query}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QueryOrder, PaginatorTrait, QuerySelect, ActiveModelTrait, Set, Order};
use uuid::Uuid;
use validator::Validate;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::chat::list_rooms_query::ListRoomsQuery;
use crate::model::requests::pagination::PaginationRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::responses::chat::room_response::ChatRoomResponse;
use crate::entities::{chat_rooms, chat_room_members, users};

#[utoipa::path(
    get, path = "/api/chat/rooms", params(ListRoomsQuery),
    responses(
        (status = 200, description = "List of chat rooms for the authenticated user", body = ApiResponse<Vec<ChatRoomResponse>>),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_rooms(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<ListRoomsQuery>,
) -> Result<Json<ApiResponse<Vec<ChatRoomResponse>>>, AppError> {
    let PaginationRequest { page, limit } = query.pagination;
    let user_id = auth.user.id;

    // Find all room IDs where the user is a member
    let member_room_ids: Vec<Uuid> = chat_room_members::Entity::find()
        .filter(chat_room_members::Column::UserId.eq(user_id))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| m.room_id)
        .collect();

    if member_room_ids.is_empty() {
        let pagination = PaginationResponse::new(limit, page, 0);
        return Ok(Json(ApiResponse::success_with_meta(200, "Chat rooms retrieved successfully", vec![], pagination)));
    }

    let base_query = chat_rooms::Entity::find()
        .filter(chat_rooms::Column::Id.is_in(member_room_ids.clone()))
        .order_by_desc(chat_rooms::Column::CreatedAt);

    let total_records = base_query.clone().count(db.as_ref()).await?;

    let rooms: Vec<chat_rooms::Model> = base_query
        .offset((page - 1) * limit)
        .limit(limit)
        .all(db.as_ref())
        .await?;

    // Batch-fetch creators and member counts
    let creator_ids: Vec<Uuid> = rooms.iter().map(|r| r.created_by).collect();
    let creators = users::Entity::find()
        .filter(users::Column::Id.is_in(creator_ids))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|u| (u.id, u))
        .collect::<std::collections::HashMap<_, _>>();

    let data: Vec<ChatRoomResponse> = rooms
        .into_iter()
        .map(|room| {
            let creator = creators.get(&room.created_by).cloned();
            let member_count = member_room_ids.iter().filter(|&&rid| rid == room.id).count() as u64;
            // TODO: fetch last message from MongoDB for preview
            crate::services::v1::chat::rooms::map_to_room_response(room, creator, member_count, None)
        })
        .collect();

    let pagination = PaginationResponse::new(limit, page, total_records);
    Ok(Json(ApiResponse::success_with_meta(200, "Chat rooms retrieved successfully", data, pagination)))
}

#[utoipa::path(
    post, path = "/api/chat/rooms",
    responses(
        (status = 201, description = "Chat room created", body = ApiResponse<ChatRoomResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_room(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<ChatRoomResponse>>, AppError> {
    let now = chrono::Utc::now();
    let room_id = Uuid::new_v4();

    let room = chat_rooms::ActiveModel {
        id: Set(room_id),
        room_name: Set(format!("Chat with {}", auth.user.full_name)),
        created_by: Set(auth.user.id),
        created_at: Set(now),
    };
    room.insert(db.as_ref()).await?;

    // Add creator as first member
    let member = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(auth.user.id),
        joined_at: Set(now),
    };
    member.insert(db.as_ref()).await?;

    let response = crate::services::v1::chat::rooms::map_to_room_response(
        chat_rooms::Model {
            id: room_id,
            room_name: format!("Chat with {}", auth.user.full_name),
            created_by: auth.user.id,
            created_at: now,
        },
        Some(auth.user.clone()),
        1,
        None,
    );

    Ok(Json(ApiResponse::success(201, "Chat room created successfully", response)))
}
