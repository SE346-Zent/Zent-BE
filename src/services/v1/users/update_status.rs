use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserStatusUpdateRequest,
};

/// Represents the side-effects for updating a user's status.
#[derive(Debug)]
pub struct UpdateStatusEffect {
    pub user_active_model: users::ActiveModel,
}

/// Validate and prepare status update.
pub fn decide_can_update_status(_current_user: users::Model, _req: UserStatusUpdateRequest) -> Result<UpdateStatusEffect, AppError> {
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
    const ROLE_TECHNICIAN: i32 = 4;

    #[fixture]
    fn mock_user(#[default(1)] role_id: i32) -> users::Model {
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
    #[case(ROLE_SUPER_ADMIN, 2, "ok")]
    #[case(ROLE_ADMIN, 3, "ok")]
    #[case(ROLE_TECHNICIAN, 1, "forbidden")]
    fn test_decide_can_update_status_rbac(#[case] role_id: i32, #[case] target_status: i32, #[case] expected: &str) {
        let user = mock_user(role_id);
        let req = UserStatusUpdateRequest { account_status_id: target_status };
        let res = decide_can_update_status(user, req);
        
        match expected {
            "ok" => {
                let effect = res.expect("Should be OK");
                assert_eq!(effect.user_active_model.account_status, Set(target_status));
            },
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }
}
