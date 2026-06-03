use crate::{
    core::errors::AppError, entities::users,
    model::requests::auth::forgot_password_request::ForgotPasswordRequest, utils::otp,
};

/// Represents the calculated results and side-effects of a forgotten password request.
pub struct ForgotPasswordEffect {
    /// The user's email address to which the recovery OTP will be sent.
    pub email_address: String,
    /// The user's full name for email personalization.
    pub full_name: String,
    /// The generated 6-digit recovery OTP code.
    pub recovery_otp: String,
}

/// Determine the outcome of a forgot password request based on user existence.
///
/// This pure function validates if a user exists for the given email and, if so,
/// generates a recovery OTP and prepares the side-effect data.
///
/// # Arguments
/// * `user_record` - An optional database model of the user matching the email.
/// * `forgot_password_payload` - The raw request payload containing the email address.
///
/// # Returns
/// A result containing an optional `ForgotPasswordEffect` (if user exists), or an `AppError`.
pub fn decide_forgot_password(
    user_record: Option<&users::Model>,
    forgot_password_payload: ForgotPasswordRequest,
) -> Result<Option<ForgotPasswordEffect>, AppError> {
    match user_record {
        Some(user) => {
            let recovery_otp = otp::generate_6digit_otp();
            Ok(Some(ForgotPasswordEffect {
                email_address: forgot_password_payload.email,
                full_name: user.full_name.clone(),
                recovery_otp,
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
    fn test_decide_forgot_password_user_exists(#[case] email_address: String, #[case] account_status: i32) {
        let user_record = users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: email_address.clone(),
            password_hash: "hash".to_string(),
            phone_number: "123".to_string(),
            account_status: account_status,
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            recovery_email: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        };
        let payload = ForgotPasswordRequest {
            email: "john@example.com".to_string(),
            use_recovery_email: None,
        };

        let result = decide_forgot_password(Some(&user_record), payload);
        assert!(result.is_ok());
        let effect = result.unwrap().unwrap();
        assert_eq!(effect.email_address, "john@example.com");
        assert_eq!(effect.full_name, "John Doe");
        assert_eq!(effect.recovery_otp.len(), 6);
    }

    #[test]
    fn test_decide_forgot_password_user_missing() {
        let payload = ForgotPasswordRequest {
            email: "missing@example.com".to_string(),
            use_recovery_email: None,
        };

        let result = decide_forgot_password(None, payload);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
