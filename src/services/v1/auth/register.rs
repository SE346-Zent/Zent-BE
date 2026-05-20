use crate::utils::otp;
use crate::{
    core::errors::AppError, entities::users,
    model::requests::auth::user_registration_request::UserRegistrationRequest,
};
use uuid::Uuid;

/// Represents the calculated results and side-effects of a user registration attempt.
///
/// This structure decouples the business logic of registration (e.g., OTP generation, 
/// status assignment) from the infrastructure tasks like persistence and email delivery.
pub struct RegisterEffect {
    /// Unique identifier for the registering user.
    pub user_id: Uuid,
    /// User's full name.
    pub full_name: String,
    /// User's email address.
    pub email_address: String,
    /// User's phone number.
    pub phone_number: String,
    /// The role ID assigned to the user (usually 'Customer').
    pub role_id: i32,
    /// The initial account status ID assigned to the user (usually 'Pending').
    pub account_status: i32,
    /// The hashed version of the user's password.
    pub hashed_password: String,
    /// Boolean indicating if this is a brand new user record or a retry for a pending user.
    pub is_new_record: bool,
    /// The generated 6-digit OTP code for email verification.
    pub verification_otp: String,
}

/// Determine the outcome of a registration attempt based on existing user state and request data.
///
/// This pure function validates if the email is already in use by an active account
/// and prepares the data for a new or updated user record and verification OTP.
///
/// # Arguments
/// * `registration_request` - The validated registration request payload.
/// * `existing_user_record` - An optional database model of an existing user with the same email.
/// * `pending_status_id` - The database ID representing the 'Pending' account status.
/// * `customer_role_id` - The database ID representing the 'Customer' role.
/// * `hashed_password` - The already-hashed password string.
///
/// # Returns
/// A result containing the `RegisterEffect` on success, or a `Conflict` error if the email is taken.
pub fn decide_register(
    registration_request: UserRegistrationRequest,
    existing_user_record: Option<&users::Model>,
    pending_status_id: i32,
    customer_role_id: i32,
    hashed_password: String,
) -> Result<RegisterEffect, AppError> {
    // 1. Check existing user
    if let Some(user) = existing_user_record {
        if user.account_status != pending_status_id {
            return Err(AppError::Conflict(
                "Email already registered and active".to_string(),
            ));
        }
    }

    // 2. Prepare user ID
    let is_new_record = existing_user_record.is_none();
    let user_id = if let Some(u) = existing_user_record {
        u.id
    } else {
        Uuid::new_v4()
    };

    // 3. OTP
    let verification_otp = otp::generate_6digit_otp();

    Ok(RegisterEffect {
        user_id,
        full_name: registration_request.full_name,
        email_address: registration_request.email,
        phone_number: registration_request.phone_number,
        role_id: customer_role_id,
        account_status: pending_status_id,
        hashed_password,
        is_new_record,
        verification_otp,
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
            avatar_url: None,
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
        let existing_user_record = existing_status.map(|status| mock_user(status));
        let email_address = mock_request.email.clone();
        let result = decide_register(
            mock_request,
            existing_user_record.as_ref(),
            pending_status_id,
            customer_role_id,
            "hashed".to_string(),
        );

        match expected_result {
            "Ok" => {
                assert!(result.is_ok());
                let registration_effect = result.unwrap();
                assert_eq!(registration_effect.email_address, email_address);
                assert_eq!(registration_effect.is_new_record, expected_is_new);
                if let Some(user) = existing_user_record {
                    assert_eq!(registration_effect.user_id, user.id);
                }
            }
            "Conflict" => {
                assert!(matches!(result, Err(AppError::Conflict(_))));
            }
            _ => panic!("Unknown expected result type"),
        }
    }
}
