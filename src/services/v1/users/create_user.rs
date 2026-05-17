use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserCreateRequest,
};

/// Represents the side-effects for creating a new user.
#[derive(Debug)]
pub struct CreateUserEffect {
    pub user_active_model: users::ActiveModel,
    pub plain_password: Option<String>,
}

/// Validate and prepare user creation.
pub fn decide_can_create_user(_current_user: users::Model, _req: UserCreateRequest) -> Result<CreateUserEffect, AppError> {
    unimplemented!()
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
    // SA can create everyone
    #[case(ROLE_SUPER_ADMIN, ROLE_SUPER_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_ADMIN, "ok")]
    #[case(ROLE_SUPER_ADMIN, ROLE_TECHNICIAN, "ok")]
    // Admin can create Tech and Customer
    #[case(ROLE_ADMIN, ROLE_TECHNICIAN, "ok")]
    #[case(ROLE_ADMIN, ROLE_CUSTOMER, "ok")]
    // Admin CANNOT create Admin or SA
    #[case(ROLE_ADMIN, ROLE_ADMIN, "forbidden")]
    #[case(ROLE_ADMIN, ROLE_SUPER_ADMIN, "forbidden")]
    // Others can't create anyone
    #[case(ROLE_TECHNICIAN, ROLE_CUSTOMER, "forbidden")]
    fn test_decide_can_create_user_rbac(#[case] current_role: i32, #[case] target_role: i32, #[case] expected: &str) {
        let current_user = mock_user(current_role);
        let req = UserCreateRequest {
            role_id: target_role,
            full_name: "New".to_string(),
            email: "new@zent.com".to_string(),
            phone: None,
            password: None,
            generate_password: Some(true),
        };
        let res = decide_can_create_user(current_user, req);
        
        match expected {
            "ok" => {
                let effect = res.expect("Should be OK");
                assert_eq!(effect.user_active_model.role_id, Set(target_role));
            },
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }
}
