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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_room_response_full() {
        let room_id = Uuid::new_v4();
        let resp = map_to_room_response(
            room_id,
            "Alice".to_string(),
            Some("https://img.example.com/alice.jpg".to_string()),
            Some("Hey, how are you?".to_string()),
            Some("2026-01-15T10:30:00Z".to_string()),
            3,
        );
        assert_eq!(resp.id, room_id);
        assert_eq!(resp.opposite_user_name, "Alice");
        assert_eq!(resp.opposite_avatar_url, Some("https://img.example.com/alice.jpg".to_string()));
        assert_eq!(resp.latest_message, Some("Hey, how are you?".to_string()));
        assert_eq!(resp.latest_message_at, Some("2026-01-15T10:30:00Z".to_string()));
        assert_eq!(resp.unread_count, 3);
    }

    #[test]
    fn test_map_room_response_minimal() {
        let room_id = Uuid::new_v4();
        let resp = map_to_room_response(
            room_id,
            "Bob".to_string(),
            None,
            None,
            None,
            0,
        );
        assert_eq!(resp.id, room_id);
        assert_eq!(resp.opposite_user_name, "Bob");
        assert_eq!(resp.opposite_avatar_url, None);
        assert_eq!(resp.latest_message, None);
        assert_eq!(resp.latest_message_at, None);
        assert_eq!(resp.unread_count, 0);
    }

    #[test]
    fn test_map_room_response_image_preview_overrides_content() {
        let room_id = Uuid::new_v4();
        let resp = map_to_room_response(
            room_id,
            "Carol".to_string(),
            None,
            Some("Alice has sent an image".to_string()),
            Some("2026-01-15T12:00:00Z".to_string()),
            1,
        );
        assert_eq!(resp.latest_message, Some("Alice has sent an image".to_string()));
    }
}
