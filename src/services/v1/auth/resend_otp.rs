use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    entities::{account_status, users},
    model::{
        requests::auth::resend_otp_request::ResendOtpRequest,
        responses::base::ApiResponse,
    },
    services::v1::core::email_service,
};
use lapin::Connection;
use crate::utils::otp;
use redis::AsyncCommands;

/// Describes the outcome of a resend OTP request.
pub enum ResendOtpIntent {
    SendOtp {
        email: String,
        full_name: String,
        otp_code: String,
    },
    Error(AppError),
}

/// Pure logic to decide the outcome of a resend OTP request.
pub fn decide_resend_otp(
    user_model: Option<users::Model>,
    pending_status_id: i32,
    req: ResendOtpRequest,
) -> ResendOtpIntent {
    let user = match user_model {
        Some(u) => u,
        None => return ResendOtpIntent::Error(AppError::NotFound("User not found".to_string())),
    };

    if user.account_status != pending_status_id {
        return ResendOtpIntent::Error(AppError::BadRequest("Account is not pending".to_string()));
    }

    let verification_code = otp::generate_6digit_otp();

    ResendOtpIntent::SendOtp {
        email: req.email,
        full_name: user.full_name,
        otp_code: verification_code,
    }
}

pub async fn handle_resend_otp(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    req: ResendOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Fetch data (I/O)
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?;
    
    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;

    // 2. Decision Logic (Pure)
    let intent = decide_resend_otp(user, pending_status.id, req);

    // 3. Execution (I/O)
    match intent {
        ResendOtpIntent::SendOtp { email, full_name, otp_code } => {
            if let Some(mut conn) = valkey {
                let valkey_key = format!("register_verification:{}", email);
                let valkey_data = serde_json::json!({ "code": otp_code, "attempts": 5 }).to_string();
                conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
            }

            if let Some(rmq) = rabbitmq {
                email_service::send_verification_email(&rmq, templates, &email, &full_name, &otp_code).await?;
            }

            Ok(ApiResponse::message_only(200, "OTP resent"))
        }
        ResendOtpIntent::Error(err) => Err(err),
    }
}
