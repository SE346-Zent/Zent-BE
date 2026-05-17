use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserStatusUpdateRequest,
};

/// Validate if the current user is allowed to update another user's account status.
pub fn decide_can_update_status(_current_user: &users::Model, _req: &UserStatusUpdateRequest) -> Result<(), AppError> {
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
    #[case(ROLE_SUPER_ADMIN, "ok")]
    #[case(ROLE_ADMIN, "ok")]
    #[case(ROLE_TECHNICIAN, "forbidden")]
    fn test_decide_can_update_status_rbac(#[case] role_id: i32, #[case] expected: &str) {
        let user = mock_user(role_id);
        let req = UserStatusUpdateRequest { account_status_id: 2 };
        let res = decide_can_update_status(&user, &req);
        
        match expected {
            "ok" => assert!(res.is_ok()),
            "forbidden" => assert!(matches!(res, Err(AppError::Forbidden(_)))),
            _ => panic!("Unknown expected state"),
        }
    }

    #[rstest]
    fn test_decide_can_update_status_self_update() {
        let admin = mock_user(ROLE_ADMIN);
        let req = UserStatusUpdateRequest { account_status_id: 3 }; // Locked
        // Should we allow admins to lock themselves? 
        // Logic should probably prevent this.
        let res = decide_can_update_status(&admin, &req);
        assert!(res.is_err());
    }
}
