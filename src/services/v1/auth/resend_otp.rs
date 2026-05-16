use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::auth::resend_otp_request::ResendOtpRequest,
};
use crate::utils::otp;

/// Plain struct representing the side-effects that need to be persisted
pub struct ResendOtpEffect {
    pub email: String,
    pub full_name: String,
    pub otp_code: String,
}

/// Pure logic to decide the outcome of a resend OTP request.
pub fn decide_resend_otp(
    user_model: Option<&users::Model>,
    pending_status_id: i32,
    _req: ResendOtpRequest,
) -> Result<ResendOtpEffect, AppError> {
    let user = match user_model {
        Some(u) => u,
        None => return Err(AppError::NotFound("User not found".to_string())),
    };

    if user.account_status != pending_status_id {
        return Err(AppError::BadRequest("Account is not pending".to_string()));
    }

    let verification_code = otp::generate_6digit_otp();

    Ok(ResendOtpEffect {
        email: user.email.clone(),
        full_name: user.full_name.clone(),
        otp_code: verification_code,
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
        let user = existing_status.map(|status| mock_user(status));
        let req = ResendOtpRequest {
            email: user.as_ref().map(|u| u.email.clone()).unwrap_or_else(|| "missing@example.com".to_string()),
        };

        let result = decide_resend_otp(user.as_ref(), 1, req);

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let effect = result.unwrap();
                let mock_u = user.unwrap();
                assert_eq!(effect.email, mock_u.email);
                assert_eq!(effect.full_name, mock_u.full_name);
                assert_eq!(effect.otp_code.len(), 6);
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
