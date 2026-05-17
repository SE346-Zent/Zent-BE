use uuid::Uuid;
use crate::model::responses::chat::room_response::ChatRoomResponse;

/// Pure logic: maps room data + opposite user info + last message + unread count → response DTO.
pub fn map_to_room_response(
    room_id: Uuid,
    opposite_user_name: String,
    opposite_avatar_url: Option<String>,
    latest_message: Option<String>,
    latest_message_at: Option<String>,
    unread_count: u64,
) -> ChatRoomResponse {
    ChatRoomResponse {
        id: room_id,
        opposite_user_name,
        opposite_avatar_url,
        latest_message,
        latest_message_at,
        unread_count,
    }
}
