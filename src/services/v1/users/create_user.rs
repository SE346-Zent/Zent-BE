use crate::{core::errors::AppError, entities::users, model::requests::users::UserCreateRequest};

/// Validate if the current user is allowed to create a new user with the given role.
pub fn decide_can_create_user(
    _current_user: &users::Model,
    _req: &UserCreateRequest,
) -> Result<(), AppError> {
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

    fn req(role: i32) -> UserCreateRequest {
        UserCreateRequest {
            role_id: role,
            first_name: "New".to_string(),
            last_name: "User".to_string(),
            email: "new@zent.com".to_string(),
            phone: None,
            password: None,
            generate_password: Some(true),
        }
    }

    #[rstest]
    // Super Admin can create anyone
    #[case(ROLE_SUPER_ADMIN, ROLE_SUPER_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_TECHNICIAN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_CUSTOMER, "ok")]
    // Admin can create Tech and Customer, but NOT Admin or SA
    #[case(ROLE_ADMIN, ROLE_TECHNICIAN, "ok")]
    #[case(ROLE_ADMIN, ROLE_CUSTOMER, "ok")]
    #[case(ROLE_ADMIN, ROLE_ADMIN, "forbidden")]
    #[case(ROLE_ADMIN, ROLE_SUPER_ADMIN, "forbidden")]
    // Others can't create anyone
    #[case(ROLE_TECHNICIAN, ROLE_CUSTOMER, "forbidden")]
    #[case(ROLE_CUSTOMER, ROLE_CUSTOMER, "forbidden")]
    fn test_decide_can_create_user_rbac(
        #[case] current_role: i32,
        #[case] target_role: i32,
        #[case] expected: &str,
    ) {
        let current_user = mock_user(current_role);
        let request = req(target_role);
        let res = decide_can_create_user(&current_user, &request);

        match expected {
            "ok" => assert!(res.is_ok()),
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!("Unknown expected state"),
        }
    }

    #[rstest]
    fn test_decide_can_create_user_invalid_email() {
        let admin = mock_user(ROLE_ADMIN);
        let mut request = req(ROLE_TECHNICIAN);
        request.email = "invalid-email".to_string();
        // Validation logic should handle this
        let res = decide_can_create_user(&admin, &request);
        assert!(res.is_err());
    }
}
