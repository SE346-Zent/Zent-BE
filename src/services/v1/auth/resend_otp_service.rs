use crate::{
    core::errors::AppError,
    model::responses::base::ApiResponse,
    model::requests::auth::resend_otp_request::ResendOtpRequest,
    repository::{account_status_repository, user_repository},
    services::v1::core::email_service,
};
use sea_orm::*;
use redis::AsyncCommands;
use rand::Rng;

pub async fn perform_resend_otp(
    db: DatabaseConnection,
    valkey: redis::Client,
    rabbitmq: std::sync::Arc<lapin::Connection>,
    req: ResendOtpRequest,
) -> Result<ApiResponse<()>, AppError> {
    let user_model = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    let pending_status = account_status_repository::find_by_name(&db, "Pending")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

    if user_model.account_status != pending_status.id {
        return Err(AppError::BadRequest("Account is already verified or not in pending state".to_string()));
    }

    let verification_code_str = {
        let mut rng = rand::rng();
        let verification_code: u32 = rng.random_range(100_000..=999_999);
        verification_code.to_string()
    };

    let valkey_key = format!("register_verification:{}", req.email);
    let valkey_data = serde_json::json!({
        "code": verification_code_str,
        "attempts": 5
    }).to_string();

    let mut valkey_conn = valkey.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
    valkey_conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 300).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store verification code in Valkey: {}", e)))?;

    email_service::send_verification_email(
        &rabbitmq,
        &req.email,
        &user_model.full_name,
        &verification_code_str,
    ).await?;

    Ok(ApiResponse::message_only(200, "OTP resent successfully"))
}
