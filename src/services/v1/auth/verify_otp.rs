use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    entities::{account_status, users},
    model::{
        requests::auth::verify_otp_request::VerifyOtpRequest,
        responses::base::ApiResponse,
    },
    services::v1::core::email_service,
};
use lapin::Connection;

pub async fn handle_verify_otp(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    script_hashes: &HashMap<String, String>,
    req: VerifyOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("register_verification:{}", req.email);
    let result: i32 = redis::cmd("EVALSHA").arg(script_hash).arg(1).arg(&valkey_key).arg(&req.otp_code)
        .query_async(&mut conn).await?;

    match result {
        1 => {} // Success
        -1 => return Err(AppError::BadRequest("OTP expired or invalid".to_string())),
        -2 => return Err(AppError::BadRequest("Invalid OTP".to_string())),
        -3 => return Err(AppError::Forbidden("Too many attempts".to_string())),
        _ => return Err(AppError::Internal(anyhow::anyhow!("Unexpected result: {}", result))),
    }

    // Update to Active
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    let active_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Active"))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing")))?;

    let mut user_active: users::ActiveModel = user.clone().into();
    user_active.account_status = Set(active_status.id);
    user_active.update(&db).await?;

    if let Some(rmq) = rabbitmq {
        email_service::send_welcome_email(&rmq, templates, &user.email, &user.full_name).await?;
    }

    Ok(ApiResponse::message_only(200, "Verified successfully"))
}
