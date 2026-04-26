use crate::{
    core::errors::AppError,
    model::{
        responses::base::ApiResponse,
        responses::auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
    },
    model::requests::auth::verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
};
use redis::AsyncCommands;
use uuid::Uuid;

pub async fn perform_verify_forgot_password_otp(
    valkey: redis::Client,
    req: VerifyForgotPasswordOtpRequest,
) -> Result<ApiResponse<VerifyForgotPasswordOtpResponseData>, AppError> {
    let valkey_key = format!("forgot_password_verification:{}", req.email);
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
        return Err(AppError::Forbidden("Too many failed attempts. Please request a new OTP.".to_string()));
    }

    if req.otp_code != stored_code {
        attempts -= 1;
        if attempts == 0 {
            let _ = valkey_conn.del::<_, ()>(&valkey_key).await;
            return Err(AppError::Forbidden("Too many failed attempts. Please request a new OTP.".to_string()));
        } else {
            parsed_data["attempts"] = serde_json::json!(attempts);
            let updated_valkey_data = parsed_data.to_string();
            let ttl: i64 = valkey_conn.ttl(&valkey_key).await.unwrap_or(600);
            if ttl > 0 {
                valkey_conn.set_ex::<_, _, ()>(&valkey_key, updated_valkey_data, ttl as u64).await
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update verification code in Valkey: {}", e)))?;
            }
            return Err(AppError::BadRequest("Invalid OTP".to_string()));
        }
    }

    // OTP matches, delete from cache
    let _ = valkey_conn.del::<_, ()>(&valkey_key).await;

    // Generate a reset token
    let reset_token = Uuid::new_v4().to_string();
    let reset_token_key = format!("password_reset_token:{}", reset_token);
    
    // Store email associated with this token
    valkey_conn.set_ex::<_, _, ()>(&reset_token_key, &req.email, 900).await // 15 minutes
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store reset token in Valkey: {}", e)))?;

    Ok(ApiResponse::success(200, "OTP verified successfully", VerifyForgotPasswordOtpResponseData { reset_token }))
}
