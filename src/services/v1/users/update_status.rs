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
    #[case(2, "ok")] // SA
    #[case(1, "ok")] // Admin
    #[case(4, "forbidden")] // Tech
    fn test_decide_can_update_status_rbac(#[case] role_id: i32, #[case] expected: &str) {
        let user = mock_user(role_id);
        let req = UserStatusUpdateRequest { account_status_id: 2 };
        let res = decide_can_update_status(user, req);
        
        match expected {
            "ok" => assert_eq!(res.unwrap().user_active_model.account_status, Set(2)),
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!(),
        }
    }
}
