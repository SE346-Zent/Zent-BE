use crate::{
    entities::{chat_rooms, users},
    model::responses::chat::room_response::{ChatRoomResponse, LastMessagePreview},
};

/// Pure logic: maps a chat room row + creator user + member count into a response DTO.
pub fn map_to_room_response(
    room: chat_rooms::Model,
    creator: Option<users::Model>,
    member_count: u64,
    last_message: Option<(String, String, String)>, // (content, sender_name, sent_at)
) -> ChatRoomResponse {
    ChatRoomResponse {
        id: room.id,
        room_name: room.room_name,
        created_by: room.created_by,
        created_by_name: creator
            .map(|u| u.full_name)
            .unwrap_or_else(|| "Unknown".to_string()),
        member_count,
        last_message: last_message.map(|(content, sender_name, sent_at)| {
            LastMessagePreview {
                content,
                sender_name,
                sent_at,
            }
        }),
        created_at: room.created_at,
    }
}
