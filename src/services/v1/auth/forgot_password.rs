use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::forgot_password_request::ForgotPasswordRequest,
    },
    utils::otp,
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
) -> Result<ForgotPasswordEffect, AppError> {
    match user {
        Some(user) => {
            let otp_code = otp::generate_6digit_otp();
            Ok(ForgotPasswordEffect {
                email: req.email,
                full_name: user.full_name.clone(),
                otp_code,
            })
        }
        None => Err(AppError::NotFound("User not found".to_string())),
    }
}
