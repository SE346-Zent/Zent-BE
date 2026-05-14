use crate::core::errors::AppError;
use crate::entities::part_audit_log;
use sea_orm::Set;

pub struct AcceptPartEffect {
    pub new_part_form_id: uuid::Uuid,
    pub audit: part_audit_log::ActiveModel,
}

pub fn decide_accept_part(
    new_part_form_id: uuid::Uuid,
    admin_id: uuid::Uuid,
    current_status: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<AcceptPartEffect, AppError> {
    if current_status.to_lowercase() != "pending" {
        return Err(AppError::BadRequest(format!("Cannot accept part with status '{}'; must be 'pending'", current_status)));
    }
    Ok(AcceptPartEffect {
        new_part_form_id,
        audit: part_audit_log::ActiveModel {
            id: Set(uuid::Uuid::new_v4()),
            new_part_form_id: Set(new_part_form_id),
            action: Set("approved".to_string()),
            admin_id: Set(admin_id),
            reason: Set(None),
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
    fn test_accept_pending() {
        let r = decide_accept_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "pending", chrono::Utc::now());
        assert!(r.is_ok());
        assert_eq!(r.unwrap().audit.action.unwrap(), "approved");
    }
    #[test]
    fn test_accept_rejects_non_pending() {
        let r = decide_accept_part(u("f0000000-0000-0000-0000-000000000001"), u("a0000000-0000-0000-0000-000000000001"), "approved", chrono::Utc::now());
        assert!(r.is_err());
    }
}
