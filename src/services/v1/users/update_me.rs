use crate::{
    core::errors::AppError, entities::users, model::requests::users::ProfileUpdateRequest,
};

/// Validate if the current user is allowed to update their profile.
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
    fn mock_user(#[default(false)] is_deleted: bool) -> users::Model {
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
            deleted_at: if is_deleted { Some(Utc::now()) } else { None },
        }
    }

    #[rstest]
    #[case(false, "valid")]
    #[case(true, "unauthorized")]
    fn test_decide_update_me_scenarios(#[case] is_deleted: bool, #[case] expected: &str) {
        let user = mock_user(is_deleted);
        let req = ProfileUpdateRequest {
            first_name: Some("Jane".to_string()),
            last_name: None,
            email: None,
            phone: None,
        };
        let res = decide_update_me(&user, &req);

        match expected {
            "valid" => assert!(res.is_ok()),
            "unauthorized" => assert!(matches!(res, Err(AppError::Unauthorized(_)))),
            _ => panic!("Unknown expected state"),
        }
    }

    #[rstest]
    fn test_decide_update_me_no_changes(mock_user: users::Model) {
        let req = ProfileUpdateRequest {
            first_name: None,
            last_name: None,
            email: None,
            phone: None,
        };
        assert!(decide_update_me(&mock_user, &req).is_ok());
    }
}
