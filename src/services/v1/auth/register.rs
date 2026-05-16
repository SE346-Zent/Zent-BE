use crate::utils::otp;
use crate::{
    core::errors::AppError, entities::users,
    model::requests::auth::user_registration_request::UserRegistrationRequest,
};
use uuid::Uuid;

/// Plain struct representing the side-effects that need to be persisted
pub struct RegisterEffect {
    pub user_id: Uuid,
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub role_id: i32,
    pub account_status: i32,
    pub hashed_password: String,
    pub is_new: bool,
    pub otp_code: String,
}

/// Pure logic to decide the outcome of a registration attempt.
pub fn decide_register(
    req: UserRegistrationRequest,
    existing_user: Option<&users::Model>,
    pending_status_id: i32,
    customer_role_id: i32,
    hashed_password: String,
) -> Result<RegisterEffect, AppError> {
    // 1. Check existing user
    if let Some(user) = existing_user {
        if user.account_status != pending_status_id {
            return Err(AppError::Conflict(
                "Email already registered and active".to_string(),
            ));
        }
    }

    // 2. Prepare user ID
    let is_new = existing_user.is_none();
    let user_id = if let Some(u) = existing_user {
        u.id
    } else {
        Uuid::new_v4()
    };

    // 3. OTP
    let otp_code = otp::generate_6digit_otp();

    Ok(RegisterEffect {
        user_id,
        full_name: req.full_name,
        email: req.email,
        phone_number: req.phone_number,
        role_id: customer_role_id,
        account_status: pending_status_id,
        hashed_password,
        is_new,
        otp_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use rstest::{fixture, rstest};

    #[fixture]
    fn pending_status_id() -> i32 {
        1
    }

    #[fixture]
    fn customer_role_id() -> i32 {
        1
    }

    #[fixture]
    fn mock_user(#[default(1)] status: i32) -> users::Model {
        users::Model {
            id: Uuid::new_v4(),
            full_name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            phone_number: "+1234567890".to_string(),
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

    #[fixture]
    fn mock_request() -> UserRegistrationRequest {
        UserRegistrationRequest {
            full_name: "New User".to_string(),
            email: "new@example.com".to_string(),
            password: "password123".to_string(),
            phone_number: "123456789".to_string(),
        }
    }

    #[rstest]
    #[case(None, "Ok", true)]
    #[case(Some(1), "Ok", false)] // 1 is pending
    #[case(Some(2), "Conflict", false)] // 2 is active
    #[case(Some(3), "Conflict", false)] // 3 is locked/other
    fn test_decide_register_exhaustive(
        #[case] existing_status: Option<i32>,
        #[case] expected_result: &str,
        #[case] expected_is_new: bool,
        mock_request: UserRegistrationRequest,
        pending_status_id: i32,
        customer_role_id: i32,
    ) {
        let existing_user = existing_status.map(|status| mock_user(status));
        let email = mock_request.email.clone();
        let result = decide_register(
            mock_request,
            existing_user.as_ref(),
            pending_status_id,
            customer_role_id,
            "hashed".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let effect = result.unwrap();
                assert_eq!(effect.email, email);
                assert_eq!(effect.is_new, expected_is_new);
                if let Some(user) = existing_user {
                    assert_eq!(effect.user_id, user.id);
                }
            }
            "Conflict" => {
                assert!(matches!(result, Err(AppError::Conflict(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
