use crate::{core::errors::AppError, entities::users};

/// Validate if the current user is allowed to close their own account.
pub fn decide_close_account(_user: &users::Model) -> Result<(), AppError> {
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
    #[case(ROLE_CUSTOMER, "ok")]
    #[case(ROLE_TECHNICIAN, "forbidden")]
    #[case(ROLE_ADMIN, "forbidden")]
    #[case(ROLE_SUPER_ADMIN, "forbidden")]
    fn test_decide_close_account_rbac(#[case] role_id: i32, #[case] expected: &str) {
        let user = mock_user(role_id);
        let res = decide_close_account(&user);

        match expected {
            "ok" => assert!(res.is_ok()),
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!("Unknown expected state"),
        }
    }

    #[rstest]
    fn test_decide_close_account_already_deleted() {
        let mut user = mock_user(ROLE_CUSTOMER);
        user.deleted_at = Some(Utc::now());
        // Should probably still be ok to "close" or return unauthorized
        let res = decide_close_account(&user);
        assert!(res.is_err());
    }
}
