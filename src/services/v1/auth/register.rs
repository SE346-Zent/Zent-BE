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
