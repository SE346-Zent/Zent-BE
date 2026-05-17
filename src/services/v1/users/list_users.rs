use crate::{
    core::errors::AppError,
    entities::users,
};

/// Represents the calculated results for listing users.
#[derive(Debug)]
pub struct ListUsersEffect {
    /// The list of users to return, possibly filtered by role.
    pub users: Vec<users::Model>,
    /// Total count for pagination.
    pub total: u64,
}

/// Validate and prepare user listing.
pub fn decide_can_list_users(_user: users::Model) -> Result<(), AppError> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rstest::{fixture, rstest};
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
    #[case(2, true)] // SA
    #[case(1, true)] // Admin
    #[case(3, false)] // Customer
    fn test_decide_can_list_users_rbac(#[case] role_id: i32, #[case] expected_ok: bool) {
        let user = mock_user(role_id);
        let res = decide_can_list_users(user);
        assert_eq!(res.is_ok(), expected_ok);
    }
}
