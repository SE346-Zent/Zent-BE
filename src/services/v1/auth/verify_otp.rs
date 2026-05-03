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

pub async fn handle_verify_otp(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<Connection>>,
    templates: &HashMap<String, String>,
    script_hashes: &HashMap<String, String>,
    req: VerifyOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. I/O: Interact with Valkey
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("register_verification:{}", req.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash)
        .arg(1)
        .arg(&valkey_key)
        .arg(&req.otp_code)
        .query_async(&mut conn)
        .await?;

    // 2. I/O: Fetch user and status info
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await?;
        
    let active_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Active"))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing")))?;

    // 3. Decision Logic (Pure)
    let effect = decide_verify_otp(result, user.as_ref(), active_status.id)?;

    // 4. Execution (I/O)
    let user_active = users::ActiveModel {
        id: Set(effect.user_id),
        account_status: Set(effect.active_status_id),
        ..Default::default()
    };
    user_active.update(&db).await?;

    if let Some(rmq) = rabbitmq {
        email_service::send_welcome_email(&rmq, templates, &effect.email, &effect.full_name).await?;
    }

    Ok(ApiResponse::message_only(200, "Verified successfully"))
}
