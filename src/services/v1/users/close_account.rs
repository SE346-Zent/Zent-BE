use crate::{
    core::errors::AppError,
    entities::users,
};

use sea_orm::Set;

/// Represents the side-effects for closing a user account.
#[derive(Debug)]
pub struct CloseAccountEffect {
    /// The updated database model with status set to Terminated and PII anonymized.
    pub user_active_model: users::ActiveModel,
}

/// Validate and prepare account closure.
///
/// Only Customers can close their own account. PII is anonymized while
/// the record is kept for foreign key integrity.
pub fn decide_close_account(user: users::Model, terminated_status_id: i32, customer_role_id: i32) -> Result<CloseAccountEffect, AppError> {
    // Only customers can self-close
    if user.role_id != customer_role_id {
        return Err(AppError::Forbidden("Only customers can close their own account. Technicians and admins must contact support.".to_string()));
    }

    // Reject if already terminated
    if user.account_status == terminated_status_id {
        return Err(AppError::Conflict("Account is already closed".to_string()));
    }

    let anonymized_id = &user.id.to_string()[..8];
    let user_active_model = users::ActiveModel {
        id: Set(user.id),
        account_status: Set(terminated_status_id),
        full_name: Set("Deleted User".to_string()),
        email: Set(format!("deleted_{}@deleted.local", anonymized_id)),
        phone_number: Set("0000000000".to_string()),
        password_hash: Set(String::new()),
        avatar_url: Set(None),
        fcm_token: Set(None),
        installation_id: Set(None),
        recovery_email: Set(None),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    Ok(CloseAccountEffect { user_active_model })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;
    use sea_orm::Set;

    const ROLE_ADMIN: i32 = 1;
    const ROLE_SUPER_ADMIN: i32 = 2;
    const ROLE_CUSTOMER: i32 = 3;
    const ROLE_TECHNICIAN: i32 = 4;

    const STATUS_TERMINATED: i32 = 42;

    #[fixture]
    fn mock_user(#[default(ROLE_CUSTOMER)] role_id: i32) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: 1,
            role_id,
            province: None,
            avatar_url: None,
            recovery_email: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    fn test_decide_close_account_customer_ok(mock_user: users::Model) {
        let res = decide_close_account(mock_user, STATUS_TERMINATED, ROLE_CUSTOMER);
        let effect = res.expect("Customer should be able to close");
        assert_eq!(effect.user_active_model.account_status, Set(STATUS_TERMINATED));
        // Verify PII is anonymized
        if let Set(ref name) = effect.user_active_model.full_name {
            assert_eq!(name, "Deleted User");
        } else {
            panic!("full_name should be Set");
        }
    }

    #[rstest]
    #[case(ROLE_TECHNICIAN)]
    #[case(ROLE_ADMIN)]
    #[case(ROLE_SUPER_ADMIN)]
    fn test_decide_close_account_non_customer_forbidden(#[case] role_id: i32) {
        let user = mock_user(role_id);
        let res = decide_close_account(user, STATUS_TERMINATED, ROLE_CUSTOMER);
        assert!(matches!(res, Err(AppError::Forbidden(_))));
    }

    #[rstest]
    fn test_decide_close_account_already_terminated() {
        let mut user = mock_user(ROLE_CUSTOMER);
        user.account_status = STATUS_TERMINATED;
        let res = decide_close_account(user, STATUS_TERMINATED, ROLE_CUSTOMER);
        assert!(matches!(res, Err(AppError::Conflict(_))));
    }
}
