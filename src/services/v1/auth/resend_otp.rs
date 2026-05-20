use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::auth::resend_otp_request::ResendOtpRequest,
};
use crate::utils::otp;

/// Represents the calculated results and side-effects of a successful resend OTP request.
pub struct ResendOtpEffect {
    /// The user's email address to which the new OTP will be sent.
    pub email_address: String,
    /// The user's full name for email personalization.
    pub full_name: String,
    /// The newly generated 6-digit OTP code.
    pub new_otp_code: String,
}

/// Determine the outcome of a resend OTP request by validating user existence and account status.
///
/// This pure function ensures that the user exists and their account is still in a 
/// 'Pending' state before generating a new verification OTP.
///
/// # Arguments
/// * `user_record` - An optional database model of the user requesting the new OTP.
/// * `pending_status_id` - The database ID representing the 'Pending' account status.
/// * `_resend_payload` - The raw request payload (currently unused in logic but kept for consistency).
///
/// # Returns
/// A result containing the `ResendOtpEffect` on success, or an `AppError` (e.g., `NotFound`, `BadRequest`).
pub fn decide_resend_otp(
    user_record: Option<&users::Model>,
    pending_status_id: i32,
    _resend_payload: ResendOtpRequest,
) -> Result<ResendOtpEffect, AppError> {
    let user = match user_record {
        Some(u) => u,
        None => return Err(AppError::NotFound("User not found".to_string())),
    };

    if user.account_status != pending_status_id {
        return Err(AppError::BadRequest("Account is not pending".to_string()));
    }

    let verification_code = otp::generate_6digit_otp();

    Ok(ResendOtpEffect {
        email_address: user.email.clone(),
        full_name: user.full_name.clone(),
        new_otp_code: verification_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::users;
    use crate::model::requests::auth::resend_otp_request::ResendOtpRequest;
    use chrono::Utc;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    fn mock_user(#[default(1)] status: i32) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "123".to_string(),
            account_status: status,
            role_id: 1,
            province: None,
            fcm_token: None,
            installation_id: None,
            avatar_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[rstest]
    #[case(Some(1), "Ok")] // Pending status
    #[case(Some(2), "BadRequest")] // Active status
    #[case(None, "NotFound")] // User not found
    fn test_decide_resend_otp_exhaustive(
        #[case] existing_status: Option<i32>,
        #[case] expected_result: &str,
    ) {
        let user_record = existing_status.map(|status| mock_user(status));
        let payload = ResendOtpRequest {
            email: user_record.as_ref().map(|u| u.email.clone()).unwrap_or_else(|| "missing@example.com".to_string()),
        };

        let result = decide_resend_otp(user_record.as_ref(), 1, payload);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let resend_effect = result.unwrap();
                let mock_u = user_record.unwrap();
                assert_eq!(resend_effect.email_address, mock_u.email);
                assert_eq!(resend_effect.full_name, mock_u.full_name);
                assert_eq!(resend_effect.new_otp_code.len(), 6);
            }
            "BadRequest" => {
                assert!(matches!(result, Err(AppError::BadRequest(_))));
            }
            "NotFound" => {
                assert!(matches!(result, Err(AppError::NotFound(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
