use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::auth::user_registration_request::UserRegistrationRequest,
};
use crate::utils::otp;
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
            return Err(AppError::Conflict("Email already registered and active".to_string()));
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

    fn create_mock_user(status: i32) -> users::Model {
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

    fn create_mock_request() -> UserRegistrationRequest {
        UserRegistrationRequest {
            full_name: "New User".to_string(),
            email: "new@example.com".to_string(),
            password: "password123".to_string(),
            phone_number: "123456789".to_string(),
        }
    }

    #[test]
    fn test_decide_register_new_user() {
        let req = create_mock_request();
        let result = decide_register(
            req.clone(),
            None,
            1, // pending_status_id
            1, // customer_role_id
            "hashed".to_string(),
        );

        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.email, req.email);
        assert_eq!(effect.account_status, 1);
        assert!(effect.is_new);
    }

    #[test]
    fn test_decide_register_existing_pending() {
        let req = create_mock_request();
        let user = create_mock_user(1); // pending
        
        let result = decide_register(
            req.clone(),
            Some(&user),
            1, // pending_status_id
            1, // customer_role_id
            "hashed".to_string(),
        );

        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.user_id, user.id);
        assert_eq!(effect.email, req.email);
        assert_eq!(effect.account_status, 1);
        assert!(!effect.is_new);
    }

    #[test]
    fn test_decide_register_existing_active() {
        let req = create_mock_request();
        let user = create_mock_user(2); // active
        
        let result = decide_register(
            req.clone(),
            Some(&user),
            1, // pending_status_id
            1, // customer_role_id
            "hashed".to_string(),
        );

        assert!(matches!(result, Err(AppError::Conflict(_))));
    }
}
