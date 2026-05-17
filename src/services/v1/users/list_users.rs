use crate::{core::errors::AppError, entities::users};

/// Validate if the current user is allowed to list users.
pub fn decide_can_list_users(_user: &users::Model) -> Result<(), AppError> {
    unimplemented!()
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
    fn test_decide_can_list_users_rbac(#[case] role_id: i32, #[case] expected_ok: bool) {
        let user = mock_user(role_id);
        let res = decide_can_list_users(&user);
        assert_eq!(res.is_ok(), expected_ok);
        if !expected_ok {
            assert!(matches!(res, Err(AppError::Forbidden(_))));
        }
    }

    #[rstest]
    fn test_decide_can_list_users_inactive_admin() {
        let mut admin = mock_user(ROLE_ADMIN);
        admin.account_status = 2; // Inactive
                                  // Inactive admins probably shouldn't be listing users
        let res = decide_can_list_users(&admin);
        assert!(res.is_err());
    }
}
