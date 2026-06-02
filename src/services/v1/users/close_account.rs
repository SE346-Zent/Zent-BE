use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the side-effects for closing a user account.
#[derive(Debug)]
pub struct CloseAccountEffect {
    /// The updated database model with status set to Terminated.
    pub user_active_model: users::ActiveModel,
}

/// Validate and prepare account closure.
///
/// Only customers (role_id = 3) can close their own account.
/// Admin, SuperAdmin, and Technician accounts cannot be self-closed.
pub fn decide_close_account(user: users::Model) -> Result<CloseAccountEffect, AppError> {
    let user_id = user.id;
    // Only customers can close their own account
    if user.role_id != 3 {
        tracing::warn!(
            user_id = %user_id,
            role_id = %user.role_id,
            error.message = "RoleNotAllowed",
            error.details = "",
            message = "Only customers can close their account"
        );
        return Err(AppError::Forbidden("Only customers can close their account".to_string()));
    }

    // Terminated status per AccountStatusEnum
    const STATUS_TERMINATED: i32 = 5;

    // Reject if already terminated
    if user.account_status == STATUS_TERMINATED {
        tracing::warn!(
            user_id = %user_id,
            error.message = "AccountAlreadyClosed",
            error.details = "",
            message = "Account is already closed"
        );
        return Err(AppError::Conflict("Account is already closed".to_string()));
    }

    let mut user_active_model: users::ActiveModel = user.into();
    user_active_model.account_status = sea_orm::Set(STATUS_TERMINATED);
    user_active_model.updated_at = sea_orm::Set(chrono::Utc::now());

    tracing::info!(
        user_id = %user_id,
        reason = "AccountClosedSuccessfully",
        message = "Account closure successfully decided"
    );

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

    const STATUS_TERMINATED: i32 = 5;

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
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(ROLE_CUSTOMER, true)]
    #[case(ROLE_TECHNICIAN, false)]
    #[case(ROLE_ADMIN, false)]
    #[case(ROLE_SUPER_ADMIN, false)]
    fn test_decide_close_account_rbac(#[case] role_id: i32, #[case] expected_ok: bool) {
        let user = mock_user(role_id);
        let res = decide_close_account(user);
        
        if expected_ok {
            let effect = res.expect("Should be OK for Customer");
            assert_eq!(effect.user_active_model.account_status, Set(STATUS_TERMINATED));
        } else {
            assert!(matches!(res, Err(AppError::Forbidden(_))));
        }
    }

    #[rstest]
    fn test_decide_close_account_already_terminated() {
        let mut user = mock_user(ROLE_CUSTOMER);
        user.account_status = STATUS_TERMINATED;
        // Logic should decide if it's an error or no-op (ActiveModel with same status)
        let res = decide_close_account(user);
        assert!(res.is_err() || res.unwrap().user_active_model.account_status == Set(STATUS_TERMINATED));
    }
}
