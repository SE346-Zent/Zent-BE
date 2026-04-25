use crate::{
    core::errors::AppError,
    model::responses::base::ApiResponse,
    model::requests::auth::verify_otp_request::VerifyOtpRequest,
    repository::{account_status_repository, user_repository},
    services::v1::core::email_service,
};
use sea_orm::*;
use redis::AsyncCommands;

pub async fn perform_verify_otp(
    db: DatabaseConnection,
    valkey: redis::Client,
    rabbitmq: std::sync::Arc<lapin::Connection>,
    req: VerifyOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    let valkey_key = format!("register_verification:{}", req.email);
    let mut valkey_conn = valkey.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
    let valkey_data: Option<String> = valkey_conn.get(&valkey_key).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to retrieve verification code from Valkey: {}", e)))?;

    let valkey_data_str = valkey_data.ok_or_else(|| AppError::BadRequest("OTP expired or invalid".to_string()))?;

    let mut parsed_data: serde_json::Value = serde_json::from_str(&valkey_data_str)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to parse verification code data")))?;

    let stored_code = parsed_data["code"].as_str().unwrap_or("");
    let mut attempts = parsed_data["attempts"].as_u64().unwrap_or(0);

    if attempts == 0 {
        let _ = valkey_conn.del::<_, ()>(&valkey_key).await;
        return Err(AppError::Forbidden("Too many failed attempts. Please register again or request new OTP.".to_string()));
    }

    if req.otp_code != stored_code {
        attempts -= 1;
        if attempts == 0 {
            let _ = valkey_conn.del::<_, ()>(&valkey_key).await;
            return Err(AppError::Forbidden("Too many failed attempts. Please register again or request new OTP.".to_string()));
        } else {
            parsed_data["attempts"] = serde_json::json!(attempts);
            let updated_valkey_data = parsed_data.to_string();
            // Get TTL to persist it
            let ttl: i64 = valkey_conn.ttl(&valkey_key).await.unwrap_or(300);
            if ttl > 0 {
                valkey_conn.set_ex::<_, _, ()>(&valkey_key, updated_valkey_data, ttl as u64).await
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update verification code in Valkey: {}", e)))?;
            }
            return Err(AppError::BadRequest("Invalid OTP".to_string()));
        }
    }

    // OTP matches, delete from cache
    let _ = valkey_conn.del::<_, ()>(&valkey_key).await;

    // Update user status
    let user_model = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let active_status = account_status_repository::find_by_name(&db, "Active")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status not found")))?;

    let mut active_user: crate::entities::users::ActiveModel = user_model.clone().into();
    active_user.account_status = Set(active_status.id);

    user_repository::update(&db, active_user)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update user status: {:?}", e)))?;

    // Send welcome email asynchronously
    email_service::send_welcome_email(
        &rabbitmq,
        &user_model.email,
        &user_model.full_name,
    ).await?;

    Ok(ApiResponse::message_only(200, "Account verified successfully"))
}
