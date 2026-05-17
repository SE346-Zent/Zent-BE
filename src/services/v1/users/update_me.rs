use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::users::ProfileUpdateRequest,
};

pub fn decide_update_me(_user: &users::Model, _req: &ProfileUpdateRequest) -> Result<(), AppError> {
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
    fn test_decide_update_me(mock_user: users::Model) {
        let req = ProfileUpdateRequest { first_name: None, last_name: None, email: None, phone: None };
        let _ = decide_update_me(&mock_user, &req);
    }
}
