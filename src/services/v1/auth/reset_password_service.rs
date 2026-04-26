use crate::{
    core::errors::AppError,
    model::responses::base::ApiResponse,
    model::requests::auth::reset_password_request::ResetPasswordRequest,
    repository::user_repository,
    entities::users,
};
use sea_orm::*;
use redis::AsyncCommands;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;

pub async fn perform_reset_password(
    db: DatabaseConnection,
    valkey: redis::Client,
    req: ResetPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    let reset_token_key = format!("password_reset_token:{}", req.reset_token);
    let mut valkey_conn = valkey.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
    let email: Option<String> = valkey_conn.get(&reset_token_key).await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to retrieve email from Valkey: {}", e)))?;

    let email = email.ok_or_else(|| AppError::BadRequest("Reset token expired or invalid".to_string()))?;

    // Hash the new password
    let password_bytes = req.new_password.clone().into_bytes();
    let hashed_password = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(&password_bytes, &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task")))?
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    // Find user
    let user = user_repository::find_by_email(&db, &email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Update user
    let mut user_active: users::ActiveModel = user.into();
    user_active.password_hash = Set(hashed_password);
    user_active.updated_at = Set(Utc::now());

    user_repository::update(&db, user_active)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update password: {:?}", e)))?;

    // Delete token
    let _ = valkey_conn.del::<_, ()>(&reset_token_key).await;

    Ok(ApiResponse::message_only(200, "Password reset successfully"))
}
