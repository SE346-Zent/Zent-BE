use crate::{
    entities::users,
    model::responses::users::MeResponseData,
};

/// Represents the calculated results and side-effects for retrieving the current user's profile.
#[derive(Debug)]
pub struct GetMeEffect {
    /// The profile data to be returned in the API response.
    pub response_data: MeResponseData,
}

/// Retrieve the current user's profile information.
pub fn decide_get_me(_user: users::Model) -> GetMeEffect {
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
    fn test_decide_get_me_mapping(mock_user: users::Model) {
        let effect = decide_get_me(mock_user.clone());
        let res = effect.response_data;
        
        assert_eq!(res.full_name, mock_user.full_name);
        assert_eq!(res.email, mock_user.email);
        assert_eq!(res.phone, Some(mock_user.phone_number));
    }
}
