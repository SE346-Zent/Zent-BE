use uuid::Uuid;
use super::NotificationRecord;

/// Mark every notification for a user as read.  Returns the number
/// of notifications that were actually transitioned.
pub fn mark_all_read(notifs: &mut [NotificationRecord], user_id: Uuid) -> usize {
    let mut count = 0;
    for n in notifs.iter_mut().filter(|n| n.user_id == user_id && !n.is_read) {
        n.is_read = true;
        count += 1;
    }
    count
}
