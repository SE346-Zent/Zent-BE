use uuid::Uuid;
use super::OutboxRecord;

/// Delete outbox entries that have been delivered for a given user.
/// Returns the number of entries removed.
pub fn cleanup_delivered(outbox: &mut Vec<OutboxRecord>, user_id: Uuid) -> usize {
    let initial_len = outbox.len();
    outbox.retain(|e| e.user_id != user_id || !e.delivered);
    initial_len - outbox.len()
}
