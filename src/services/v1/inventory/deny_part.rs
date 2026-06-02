use crate::core::errors::AppError;
use crate::entities::part_audit_log;
use sea_orm::Set;

/// Represents the calculated results and side-effects of a successful part denial request.
pub struct DenyPartEffect {
    /// The unique identifier of the part registration form being denied.
    pub target_part_form_id: uuid::Uuid,
    /// The database model for the audit log entry recording this denial and its reason.
    pub denial_audit_model: part_audit_log::ActiveModel,
}

/// Determine the outcome of a part denial attempt by validating the current form status and the provided reason.
///
/// This pure function ensures that only parts currently in 'pending' status can
/// be denied, and validates that the provided reason is within acceptable 
/// length boundaries (10-2000 characters).
///
/// # Arguments
/// * `target_part_form_id` - The unique identifier of the part form to be denied.
/// * `denying_admin_id` - The ID of the administrator performing the denial.
/// * `current_form_status` - The current status string of the part registration form.
/// * `denial_reason` - The textual reason for denying the part registration.
/// * `current_timestamp` - The current time used for the audit log entry.
///
/// # Returns
/// A result containing the `DenyPartEffect` on success, or a `BadRequest` error for validation failures.
pub fn decide_deny_part(
    target_part_form_id: uuid::Uuid,
    denying_admin_id: uuid::Uuid,
    current_form_status: &str,
    denial_reason: &str,
    current_timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<DenyPartEffect, AppError> {
    let trimmed_reason = denial_reason.trim();
    if trimmed_reason.len() < 10 {
        tracing::warn!(
            error.message = "DenialReasonTooShort", error.details = "",
            target_part_form_id = %target_part_form_id,
            denying_admin_id = %denying_admin_id,
            reason_length = trimmed_reason.len(),
            message = "Denial reason is too short"
        );
        return Err(AppError::BadRequest(format!("Denial reason must be at least 10 characters; got {}", trimmed_reason.len())));
    }
    if trimmed_reason.len() > 2000 {
        tracing::warn!(
            error.message = "DenialReasonTooLong", error.details = "",
            target_part_form_id = %target_part_form_id,
            denying_admin_id = %denying_admin_id,
            reason_length = trimmed_reason.len(),
            message = "Denial reason exceeds maximum length"
        );
        return Err(AppError::BadRequest(format!("Denial reason must not exceed 2000 characters; got {}", trimmed_reason.len())));
    }
    if current_form_status.to_lowercase() != "pending" {
        tracing::warn!(
            error.message = "InvalidPartFormStatus", error.details = "",
            target_part_form_id = %target_part_form_id,
            denying_admin_id = %denying_admin_id,
            current_form_status = %current_form_status,
            message = "Cannot deny part because its status is not pending"
        );
        return Err(AppError::BadRequest(format!("Cannot deny part with status '{}'; must be 'pending'", current_form_status)));
    }
    tracing::info!(
        target_part_form_id = %target_part_form_id,
        denying_admin_id = %denying_admin_id,
        message = "Successfully decided to deny part form"
    );
    Ok(DenyPartEffect {
        target_part_form_id,
        denial_audit_model: part_audit_log::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            new_part_form_id: Set(target_part_form_id),
            action: Set("denied".to_string()),
            admin_id: Set(denying_admin_id),
            reason: Set(Some(trimmed_reason.to_string())),
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
    fn test_deny_pending() {
        let result = decide_deny_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", "Quality check failed", chrono::Utc::now());
        assert!(result.is_ok());
        assert_eq!(result.unwrap().denial_audit_model.reason.unwrap().unwrap(), "Quality check failed");
    }
    #[test]
    fn test_deny_rejects_short_reason() {
        let result = decide_deny_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", "Short", chrono::Utc::now());
        assert!(result.is_err());
    }
}
