use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    model::{
        requests::auth::resend_otp_request::ResendOtpRequest,
        responses::base::ApiResponse,
    },
    repository::{user_repository, account_status_repository},
    services::v1::core::email_service,
    infrastructure::mq::RabbitMQManager,
};
use crate::utils::otp;
use redis::AsyncCommands;

pub async fn handle_resend_otp(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<RabbitMQManager>>,
    templates: &HashMap<String, String>,
    req: ResendOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user = user_repository::find_by_email(&db, &req.email).await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    let pending_status = account_status_repository::find_by_name(&db, "Pending").await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;

    if user.account_status != pending_status.id {
        return Err(AppError::BadRequest("Account is not pending".to_string()));
    }

    let verification_code = otp::generate_6digit_otp();

    if let Some(mut conn) = valkey {
        let valkey_key = format!("register_verification:{}", req.email);
        let valkey_data = serde_json::json!({ "code": verification_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, templates, &req.email, &user.full_name, &verification_code).await?;
    }

    Ok(ApiResponse::message_only(200, "OTP resent"))
}
