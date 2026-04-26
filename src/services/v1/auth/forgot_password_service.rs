use crate::{
    core::errors::AppError,
    model::responses::base::ApiResponse,
    model::requests::auth::forgot_password_request::ForgotPasswordRequest,
    repository::user_repository,
    services::v1::core::email_service,
};
use sea_orm::*;
use redis::AsyncCommands;
use rand::Rng;

pub async fn perform_forgot_password(
    db: DatabaseConnection,
    valkey: redis::Client,
    rabbitmq: std::sync::Arc<lapin::Connection>,
    req: ForgotPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Check if user exists
    let user = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // 2. Generate 6-digit OTP
    let otp_code = {
        let mut rng = rand::rng();
        let code: u32 = rng.random_range(100_000..=999_999);
        code.to_string()
    };

    // 3. Store in Valkey
    let valkey_key = format!("forgot_password_verification:{}", req.email);
    let valkey_data = serde_json::json!({
        "code": otp_code,
        "attempts": 5
    }).to_string();

    let mut valkey_conn = valkey.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
    valkey_conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await // 10 minutes
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store OTP in Valkey: {}", e)))?;

    // 4. Send email
    email_service::send_forgot_password_email(
        &rabbitmq,
        &req.email,
        &user.full_name,
        &otp_code,
    ).await?;

    Ok(ApiResponse::message_only(200, "OTP sent to email"))
}
