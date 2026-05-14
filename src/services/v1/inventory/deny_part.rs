use crate::core::errors::AppError;
use crate::entities::part_audit_log;
use sea_orm::Set;

pub struct DenyPartEffect {
    pub new_part_form_id: uuid::Uuid,
    pub audit: part_audit_log::ActiveModel,
}

pub fn decide_deny_part(
    new_part_form_id: uuid::Uuid,
    admin_id: uuid::Uuid,
    current_status: &str,
    reason: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<DenyPartEffect, AppError> {
    let trimmed = reason.trim();
    if trimmed.len() < 10 {
        return Err(AppError::BadRequest(format!("Denial reason must be at least 10 characters; got {}", trimmed.len())));
    }
    if trimmed.len() > 2000 {
        return Err(AppError::BadRequest(format!("Denial reason must not exceed 2000 characters; got {}", trimmed.len())));
    }
    if current_status.to_lowercase() != "pending" {
        return Err(AppError::BadRequest(format!("Cannot deny part with status '{}'; must be 'pending'", current_status)));
    }
    Ok(DenyPartEffect {
        new_part_form_id,
        audit: part_audit_log::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            new_part_form_id: Set(new_part_form_id),
            action: Set("denied".to_string()),
            admin_id: Set(admin_id),
            reason: Set(Some(trimmed.to_string())),
            created_at: Set(now),
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
        let r = decide_deny_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", "Quality check failed", chrono::Utc::now());
        assert!(r.is_ok());
        assert_eq!(r.unwrap().audit.reason.unwrap().unwrap(), "Quality check failed");
    }
    #[test]
    fn test_deny_rejects_short_reason() {
        let r = decide_deny_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", "Short", chrono::Utc::now());
        assert!(r.is_err());
    }
}
