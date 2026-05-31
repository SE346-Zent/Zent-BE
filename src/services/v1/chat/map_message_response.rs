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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_message_text_only() {
        let msg = map_to_message_response(
            "abc123".to_string(),
            "room-1".to_string(),
            "sender-uuid".to_string(),
            "Alice".to_string(),
            Some("Hello, world!".to_string()),
            None,
            None,
            "2026-01-15T10:00:00Z".to_string(),
            None,
            vec![],
        );
        assert_eq!(msg.id, "abc123");
        assert_eq!(msg.room_id, "room-1");
        assert_eq!(msg.sender_id, "sender-uuid");
        assert_eq!(msg.sender_name, "Alice");
        assert_eq!(msg.content, Some("Hello, world!".to_string()));
        assert_eq!(msg.image_url, None);
        assert_eq!(msg.reply_to, None);
        assert_eq!(msg.created_at, "2026-01-15T10:00:00Z");
        assert_eq!(msg.edited_at, None);
        assert!(msg.read_by.is_empty());
    }

    #[test]
    fn test_map_message_with_image() {
        let msg = map_to_message_response(
            "def456".to_string(),
            "room-2".to_string(),
            "sender-uuid-2".to_string(),
            "Bob".to_string(),
            None,
            Some("https://storage.example.com/img.png".to_string()),
            None,
            "2026-01-15T11:00:00Z".to_string(),
            None,
            vec!["reader-1".to_string()],
        );
        assert_eq!(msg.content, None);
        assert_eq!(msg.image_url, Some("https://storage.example.com/img.png".to_string()));
        assert_eq!(msg.read_by, vec!["reader-1".to_string()]);
    }

    #[test]
    fn test_map_message_reply_with_edited_at() {
        let msg = map_to_message_response(
            "ghi789".to_string(),
            "room-3".to_string(),
            "sender-uuid-3".to_string(),
            "Carol".to_string(),
            Some("Replying to your message".to_string()),
            None,
            Some("original-msg-id".to_string()),
            "2026-01-15T12:00:00Z".to_string(),
            Some("2026-01-15T12:05:00Z".to_string()),
            vec!["reader-1".to_string(), "reader-2".to_string()],
        );
        assert_eq!(msg.reply_to, Some("original-msg-id".to_string()));
        assert_eq!(msg.edited_at, Some("2026-01-15T12:05:00Z".to_string()));
        assert_eq!(msg.read_by.len(), 2);
    }
}
