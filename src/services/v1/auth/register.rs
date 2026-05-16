use crate::{
    core::errors::AppError,
    entities::users,
    model::requests::auth::user_registration_request::UserRegistrationRequest,
};
use crate::utils::otp;
use uuid::Uuid;

use sea_orm::Set;

/// Effect containing the ActiveModel ready for persistence, plus OTP metadata.
pub struct RegisterEffect {
    pub user: users::ActiveModel,
    pub is_new: bool,
    pub otp_code: String,
}

/// Pure logic to decide the outcome of a registration attempt.
/// Returns a `users::ActiveModel` ready for `.insert()` or `.update()`.
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

    let now = chrono::Utc::now();
    let user = users::ActiveModel {
        id: Set(user_id),
        full_name: Set(req.full_name),
        email: Set(req.email),
        phone_number: Set(req.phone_number),
        role_id: Set(customer_role_id),
        account_status: Set(pending_status_id),
        password_hash: Set(hashed_password),
        updated_at: Set(now),
        ..Default::default()
    };

    Ok(RegisterEffect { user, is_new, otp_code })
}
