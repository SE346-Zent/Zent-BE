use crate::{
    core::errors::AppError,
    entities::users,
};

/// Plain struct representing the side-effects that need to be persisted
pub struct VerifyOtpEffect {
    pub user_id: uuid::Uuid,
    pub active_status_id: i32,
    pub email: String,
    pub full_name: String,
}

/// Pure logic to decide the outcome of an OTP verification attempt.
pub fn decide_verify_otp(
    lua_result: i32,
    user_model: Option<&users::Model>,
    active_status_id: i32,
) -> Result<VerifyOtpEffect, AppError> {
    match lua_result {
        1 => {
            match user_model {
                Some(user) => {
                    Ok(VerifyOtpEffect {
                        user_id: user.id,
                        active_status_id,
                        email: user.email.clone(),
                        full_name: user.full_name.clone(),
                    })
                }
                None => Err(AppError::NotFound("User not found".to_string())),
            }
        }
        -1 => Err(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => Err(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => Err(AppError::Forbidden("Too many attempts".to_string())),
        _ => Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", lua_result))),
    }
}
