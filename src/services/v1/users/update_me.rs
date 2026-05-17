use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::users::ProfileUpdateRequest,
        responses::users::MeResponseData,
    },
};

/// Represents the calculated results and side-effects for updating the current user's profile.
#[derive(Debug)]
pub struct UpdateMeEffect {
    /// The updated database model ready for persistence.
    pub user_active_model: users::ActiveModel,
    /// The updated profile data to be returned in the API response.
    pub response_data: MeResponseData,
}

/// Validate and prepare the profile update.
pub fn decide_update_me(_user: users::Model, _req: ProfileUpdateRequest) -> Result<UpdateMeEffect, AppError> {
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
    fn test_decide_update_me_success(mock_user: users::Model) {
        let req = ProfileUpdateRequest {
            full_name: Some("Jane Doe".to_string()),
            email: Some("jane@example.com".to_string()),
            phone: Some("+0987654321".to_string()),
        };
        let effect = decide_update_me(mock_user, req).unwrap();
        
        assert_eq!(effect.user_active_model.full_name, Set("Jane Doe".to_string()));
        assert_eq!(effect.response_data.full_name, "Jane Doe");
    }

    #[rstest]
    fn test_decide_update_me_unauthorized_if_deleted() {
        let mut user = mock_user();
        user.deleted_at = Some(Utc::now());
        let req = ProfileUpdateRequest { full_name: None, email: None, phone: None };
        let res = decide_update_me(user, req);
        assert!(matches!(res, Err(AppError::Unauthorized(_))));
    }
}
