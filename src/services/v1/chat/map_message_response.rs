use crate::model::responses::chat::message_response::MessageResponse;

/// Pure logic: maps MongoDB message fields into a response DTO.
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
        read_by,
    }
}
