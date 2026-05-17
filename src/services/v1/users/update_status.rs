use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::UserStatusUpdateRequest,
};

pub fn decide_can_update_status(_current_user: &users::Model, _req: &UserStatusUpdateRequest) -> Result<(), AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_user() -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: 1,
            role_id: 3,
            province: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    fn test_decide_can_update_status(mock_user: users::Model) {
        let req = UserStatusUpdateRequest { account_status_id: 1 };
        let _ = decide_can_update_status(&mock_user, &req);
    }
}
