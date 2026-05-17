use crate::{
    core::errors::AppError, entities::users,
    model::requests::auth::forgot_password_request::ForgotPasswordRequest, utils::otp,
};

/// Plain struct representing the side-effects that need to be persisted
pub struct ForgotPasswordEffect {
    pub email: String,
    pub full_name: String,
    pub otp_code: String,
}

/// Pure logic for the forgot password flow.
/// Takes raw data and returns an Effect describing what to do next.
pub fn decide_forgot_password(
    user: Option<&users::Model>,
    req: ForgotPasswordRequest,
) -> Result<Option<ForgotPasswordEffect>, AppError> {
    match user {
        Some(user) => {
            let otp_code = otp::generate_6digit_otp();
            Ok(Some(ForgotPasswordEffect {
                email: req.email,
                full_name: user.full_name.clone(),
                otp_code,
            }))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::users;
    use crate::model::requests::auth::forgot_password_request::ForgotPasswordRequest;
    use chrono::Utc;
    use rstest::rstest;
    use uuid::Uuid;

    #[rstest]
    #[case("john@example.com".to_string(), 1)]
    #[case("john@example.com".to_string(), 2)]
    #[case("not_john@example.com".to_string(), 1)]
    #[case("not_john@example.com".to_string(), 2)]
    fn test_decide_forgot_password_user_exists(#[case] email: String, #[case] account_status: i32) {
        let user = users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: email.clone(),
            password_hash: "hash".to_string(),
            phone_number: "123".to_string(),
            account_status: account_status,
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };
        let req = ForgotPasswordRequest {
            email: "john@example.com".to_string(),
        };

        let result = decide_forgot_password(Some(&user), req);
        assert!(result.is_ok());
        let effect = result.unwrap().unwrap();
        assert_eq!(effect.email, "john@example.com");
        assert_eq!(effect.full_name, "John Doe");
        assert_eq!(effect.otp_code.len(), 6);
    }

    #[test]
    fn test_decide_forgot_password_user_missing() {
        let req = ForgotPasswordRequest {
            email: "missing@example.com".to_string(),
        };

        let result = decide_forgot_password(None, req);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
