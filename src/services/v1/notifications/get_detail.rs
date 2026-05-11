use uuid::Uuid;
use crate::{
    core::errors::AppError,
    model::responses::notifications::notification_detail_response::NotificationDetailResponse,
};
use super::NotificationRecord;

/// Fetch a single notification by id, enforcing user ownership.
pub fn get_detail(
    notifs: &[NotificationRecord],
    user_id: Uuid,
    notification_id: Uuid,
) -> Result<NotificationDetailResponse, AppError> {
    let n = notifs.iter()
        .find(|n| n.notification_id == notification_id && n.user_id == user_id)
        .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

    Ok(NotificationDetailResponse {
        notification_id: n.notification_id.to_string(),
        category_id: n.category_id,
        category_name: super::find_category_slug_by_id(n.category_id).unwrap_or("").to_string(),
        title: n.title.clone(),
        body: n.body.clone(),
        data: Some(n.data.clone()),
        is_read: n.is_read,
        os_notification_id: n.os_notification_id.map(|id| id.to_string()),
        created_at: n.created_at.to_string(),
    })
}
