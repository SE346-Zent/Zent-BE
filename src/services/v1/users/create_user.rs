use crate::{core::errors::AppError, entities::users, model::requests::users::UserCreateRequest};

/// Represents the side-effects for creating a new user.
#[derive(Debug)]
pub struct CreateUserEffect {
    pub user_active_model: users::ActiveModel,
    pub plain_password: Option<String>,
}

/// Validate and prepare user creation.
pub fn decide_can_create_user(
    _current_user: users::Model,
    _req: UserCreateRequest,
) -> Result<CreateUserEffect, AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use sea_orm::Set;
    use uuid::Uuid;

    #[fixture]
    fn mock_user(#[default(3)] role_id: i32) -> users::Model {
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
    #[case(2, 2, "forbidden")] // SA -> SA
    #[case(1, 4, "ok")] // Admin -> Tech
    #[case(1, 2, "forbidden")] // Admin -> SA
    #[case(1, 1, "forbidden")] // Admin -> Admin
    #[case(1, 3, "forbidden")] // Admin -> Customer
    #[case(2, 3, "forbidden")] // SuperAdmin -> Customer
    #[case(2, 1, "ok")] // SuperAdmin -> Admin
    #[case(2, 4, "ok")] // SuperAdmin -> Tech
    fn test_decide_can_create_user_rbac(
        #[case] current_role: i32,
        #[case] target_role: i32,
        #[case] expected: &str,
    ) {
        let user = mock_user(current_role);
        let req = UserCreateRequest {
            role_id: target_role,
            full_name: "New".to_string(),
            email: "new@zent.com".to_string(),
            phone: None,
            password: None,
            generate_password: Some(true),
        };
        let res = decide_can_create_user(user, req);

        match expected {
            "ok" => {
                let effect = res.unwrap();
                assert_eq!(effect.user_active_model.role_id, Set(target_role));
            }
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }
}
