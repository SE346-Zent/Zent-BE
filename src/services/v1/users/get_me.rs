use crate::{entities::users, model::responses::users::UserResponseData};

/// Retrieve the current user's profile information.
pub fn decide_get_me(_user: users::Model) -> UserResponseData {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_user(
        #[default("John Doe")] name: &str,
        #[default(3)] role_id: i32,
        #[default(1)] status: i32,
    ) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: name.to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
            account_status: status,
            role_id,
            province: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case("SingleName", "SingleName", "")]
    #[case("John Doe", "John", "Doe")]
    #[case("Multiple Name Test", "Multiple", "Name")]
    fn test_decide_get_me_mapping(
        #[case] full_name: &str,
        #[case] expected_first: &str,
        #[case] expected_last: &str,
    ) {
        let user = mock_user(full_name, 3, 1);
        let res = decide_get_me(user.clone());

        assert_eq!(res.id, user.id);
        assert_eq!(res.role_id, user.role_id);
        assert_eq!(res.email, user.email);
        assert_eq!(res.first_name, expected_first);
        assert_eq!(res.last_name, expected_last);
        assert_eq!(res.account_status_id, user.account_status);
        assert_eq!(res.created_at, "2026-05-17T12:00:00+00:00");
    }
}
