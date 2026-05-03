use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::forgot_password_request::ForgotPasswordRequest,
        responses::base::ApiResponse,
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

// Keeping handle_forgot_password for backward compatibility during transition if needed,
// but it should eventually be removed or moved to the handler layer.
// Actually, I will refactor it to use the new decide logic for now.

use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use lapin::Connection;
use redis::AsyncCommands;
use crate::services::v1::core::email_service;

pub async fn handle_forgot_password(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    req: ForgotPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Fetch data (I/O)
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?;

    // 2. Decision Logic (Pure)
    let effect = decide_forgot_password(user.as_ref(), req)?;

    // 3. Execution (I/O)
    if let Some(mut conn) = valkey {
        let valkey_key = format!("forgot_password_verification:{}", effect.email);
        let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_forgot_password_email(&rmq, templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
    }

    Ok(ApiResponse::message_only(200, "OTP sent"))
}
