use crate::core::errors::AppError;
use crate::entities::new_part_audit_log;
use sea_orm::Set;

/// Represents the calculated results and side-effects of a successful part acceptance request.
pub struct AcceptPartEffect {
    /// The unique identifier of the part registration form being accepted.
    pub target_part_form_id: uuid::Uuid,
    /// The database model for the audit log entry recording this approval.
    pub approval_audit_model: new_part_audit_log::ActiveModel,
}

/// Determine the outcome of a part acceptance attempt by validating the current form status.
///
/// This pure function ensures that only parts currently in 'pending' status can
/// be accepted, and prepares the audit trail entry for the approval.
///
/// # Arguments
/// * `target_part_form_id` - The unique identifier of the part form to be accepted.
/// * `approving_admin_id` - The ID of the administrator performing the approval.
/// * `current_form_status` - The current status string of the part registration form.
/// * `current_timestamp` - The current time used for the audit log entry.
///
/// # Returns
/// A result containing the `AcceptPartEffect` on success, or a `BadRequest` error if the status is not 'pending'.
pub fn decide_accept_part(
    target_part_form_id: uuid::Uuid,
    approving_admin_id: uuid::Uuid,
    current_form_status: &str,
    current_timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<AcceptPartEffect, AppError> {
    if current_form_status.to_lowercase() != "pending" {
        tracing::warn!(
            error.message = "InvalidPartFormStatus", error.details = "",
            target_part_form_id = %target_part_form_id,
            approving_admin_id = %approving_admin_id,
            current_form_status = %current_form_status,
            message = "Cannot accept part because its status is not pending"
        );
        return Err(AppError::BadRequest(format!("Cannot accept part with status '{}'; must be 'pending'", current_form_status)));
    }
    tracing::info!(
        target_part_form_id = %target_part_form_id,
        approving_admin_id = %approving_admin_id,
        message = "Successfully decided to accept part form"
    );
    Ok(AcceptPartEffect {
        target_part_form_id,
        approval_audit_model: new_part_audit_log::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            new_part_form_id: Set(target_part_form_id),
            action: Set("approved".to_string()),
            admin_id: Set(approving_admin_id),
            reason: Set(None),
            created_at: Set(current_timestamp),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
    #[test]
    fn test_accept_pending() {
        let result = decide_accept_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", chrono::Utc::now());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().approval_audit_model.action.unwrap(), "approved");
    }
    #[test]
    fn test_accept_rejects_non_pending() {
        let result = decide_accept_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "approved", chrono::Utc::now());
        assert!(result.is_err());
    }
}
