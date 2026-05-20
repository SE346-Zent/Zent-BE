use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the calculated results for listing users.
#[derive(Debug)]
pub struct ListUsersEffect {
    /// The list of users to return, possibly filtered by role.
    pub users: Vec<users::Model>,
    /// Total count for pagination.
    pub total: u64,
}

/// Validate and prepare user listing.
///
/// Only Admin and SuperAdmin may list users. The caller (orchestration layer)
/// is responsible for filtering by province / role before calling this function.
/// This function enforces the RBAC gate and passes through the pre-filtered results.
pub fn decide_list_users(
    current_user: users::Model,
    users: Vec<users::Model>,
    total: u64,
) -> Result<ListUsersEffect, AppError> {
    // RBAC: only Admin (1) or SuperAdmin (2) may list users.
    // Role IDs are resolved upstream via LookupTables; here we accept the raw i32.
    // The middleware already gates on Admin/SuperAdmin, but we double-check.
    match current_user.role_id {
        1 | 2 => {} // Admin or SuperAdmin
        _ => return Err(AppError::Forbidden("Only administrators can list users".to_string())),
    }

    Ok(ListUsersEffect { users, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    const ROLE_ADMIN: i32 = 1;
    const ROLE_SUPER_ADMIN: i32 = 2;
    const ROLE_CUSTOMER: i32 = 3;
    const ROLE_TECHNICIAN: i32 = 4;

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
    #[case(ROLE_SUPER_ADMIN, true)]
    #[case(ROLE_ADMIN, true)]
    #[case(ROLE_TECHNICIAN, false)]
    #[case(ROLE_CUSTOMER, false)]
    fn test_decide_list_users_rbac(#[case] role_id: i32, #[case] expected_ok: bool) {
        let current_user = mock_user(role_id);
        let models = vec![mock_user(ROLE_TECHNICIAN)];
        let res = decide_list_users(current_user, models, 1);
        
        if expected_ok {
            let effect = res.unwrap();
            assert_eq!(effect.users.len(), 1);
            assert_eq!(effect.total, 1);
        } else {
            assert!(matches!(res, Err(AppError::Forbidden(_))));
        }
    }

    #[rstest]
    fn test_decide_list_users_empty_results(#[values(ROLE_ADMIN, ROLE_SUPER_ADMIN)] role_id: i32) {
        let current_user = mock_user(role_id);
        let res = decide_list_users(current_user, vec![], 0).unwrap();
        assert_eq!(res.users.len(), 0);
        assert_eq!(res.total, 0);
    }
}
