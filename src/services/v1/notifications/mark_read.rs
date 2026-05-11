use uuid::Uuid;
use crate::core::errors::AppError;
use super::NotificationRecord;

/// Mark a single notification as read.  Returns `true` if it was
/// previously unread (i.e. this call had an effect).
pub fn mark_read(
    notifs: &mut [NotificationRecord],
    user_id: Uuid,
    notification_id: Uuid,
) -> Result<bool, AppError> {
    let n = notifs.iter_mut()
        .find(|n| n.notification_id == notification_id && n.user_id == user_id)
        .ok_or_else(|| AppError::NotFound("Notification not found".to_string()))?;

    if n.is_read {
        Ok(false)
    } else {
        n.is_read = true;
        Ok(true)
    }
}
