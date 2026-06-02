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
pub fn decide_get_me(user: users::Model) -> GetMeEffect {
    tracing::info!(
        user_id = %user.id,
        reason = "GetMeSuccessful",
        message = "Successfully retrieved current user profile data"
    );
    let response_data = MeResponseData {
        id: user.id,
        role_id: user.role_id,
        full_name: user.full_name,
        email: user.email,
        phone: Some(user.phone_number),
        province: user.province,
        account_status_id: user.account_status,
        employee_id: crate::utils::user::get_employee_id(user.role_id, user.id),
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    };
    GetMeEffect { response_data }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Utc, TimeZone};
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_user(
        #[default("John Doe")] name: &str,
        #[default("john@example.com")] email: &str,
        #[default("+1234567890")] phone: &str,
    ) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: name.to_string(),
            email: email.to_string(),
            password_hash: "hash".to_string(),
            phone_number: phone.to_string(),
            account_status: 1,
            role_id: 3,
            province: None,
            avatar_url: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 5, 17, 12, 0, 0).unwrap(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case("John Doe", "john@zent.com", "+111")]
    #[case("Jane Smith", "jane@zent.com", "+222")]
    #[case("OnlyName", "only@zent.com", "+333")]
    fn test_decide_get_me_mapping(
        #[case] name: &str,
        #[case] email: &str,
        #[case] phone: &str,
    ) {
        let user = mock_user(name, email, phone);
        let effect = decide_get_me(user.clone());
        let res = effect.response_data;
        
        assert_eq!(res.full_name, name);
        assert_eq!(res.email, email);
        assert_eq!(res.phone, Some(phone.to_string()));
    }
}
