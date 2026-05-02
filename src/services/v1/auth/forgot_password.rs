use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::forgot_password_request::ForgotPasswordRequest,
        responses::base::ApiResponse,
    },
    utils::otp,
};

/// Describes the side effects required after the forgot password decision logic.
pub enum ForgotPasswordIntent {
    SendOtp {
        email: String,
        full_name: String,
        otp_code: String,
    },
    Error(AppError),
}

/// Pure logic for the forgot password flow.
/// Takes raw data and returns an Intent describing what to do next.
pub fn decide_forgot_password(
    user: Option<users::Model>,
    req: ForgotPasswordRequest,
) -> ForgotPasswordIntent {
    match user {
        Some(user) => {
            let otp_code = otp::generate_6digit_otp();
            ForgotPasswordIntent::SendOtp {
                email: req.email,
                full_name: user.full_name,
                otp_code,
            }
        }
        None => ForgotPasswordIntent::Error(AppError::NotFound("User not found".to_string())),
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
    let intent = decide_forgot_password(user, req);

    // 3. Execution (I/O)
    match intent {
        ForgotPasswordIntent::SendOtp { email, full_name, otp_code } => {
            if let Some(mut conn) = valkey {
                let valkey_key = format!("forgot_password_verification:{}", email);
                let valkey_data = serde_json::json!({ "code": otp_code, "attempts": 5 }).to_string();
                conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
            }

            if let Some(rmq) = rabbitmq {
                email_service::send_forgot_password_email(&rmq, templates, &email, &full_name, &otp_code).await?;
            }

            Ok(ApiResponse::message_only(200, "OTP sent"))
        }
        ForgotPasswordIntent::Error(err) => Err(err),
    }
}
