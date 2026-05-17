use crate::model::responses::chat::message_response::{MessageResponse, ReactionEntry};

/// Pure logic: maps a MongoDB message document into a response DTO.
/// The handler is responsible for fetching sender names and read receipts from the DB.
pub fn map_to_message_response(
    id: String,
    room_id: String,
    sender_id: String,
    sender_name: String,
    content: Option<String>,
    image_url: Option<String>,
    reply_to: Option<String>,
    created_at: String,
    edited_at: Option<String>,
    reactions: Vec<ReactionEntry>,
    read_by: Vec<String>,
) -> MessageResponse {
    MessageResponse {
        id,
        room_id,
        sender_id,
        sender_name,
        content,
        image_url,
        reply_to,
        created_at,
        edited_at,
        reactions,
        read_by,
    }
}
