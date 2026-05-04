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
