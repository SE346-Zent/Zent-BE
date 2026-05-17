use axum::{extract::State, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, Set};
use uuid::Uuid;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::chat::room_response::ChatRoomResponse;
use crate::entities::{chat_rooms, chat_room_members};

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
        room_name: Set(format!("Chat {}", auth.user.full_name)),
        work_order_id: Set(None),
        created_by: Set(auth.user.id),
        created_at: Set(now),
        updated_at: Set(None),
        deleted_at: Set(None),
    };
    room.insert(db.as_ref()).await?;

    let member = chat_room_members::ActiveModel {
        room_id: Set(room_id),
        user_id: Set(auth.user.id),
        created_at: Set(now),
        updated_at: Set(None),
        deleted_at: Set(None),
    };
    member.insert(db.as_ref()).await?;

    let response = ChatRoomResponse {
        id: room_id,
        opposite_user_name: String::new(),
        opposite_avatar_url: None,
        latest_message: None,
        latest_message_at: None,
        unread_count: 0,
    };

    Ok(Json(ApiResponse::success(201, "Chat room created successfully", response)))
}
