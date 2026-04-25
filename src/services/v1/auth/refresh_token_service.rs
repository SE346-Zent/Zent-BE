use crate::entities::sessions;
use crate::core::errors::AppError;
use crate::model::requests::auth::refresh_token_request::RefreshTokenRequest;
use crate::model::responses::auth::login_response::{
    AccountStatusEnum, LoginResponseData, UserInfo,
};
use crate::model::responses::base::ApiResponse;
use crate::repository::{session_repository, user_repository};
use crate::services::v1::core::token_service;
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use sea_orm::*;
use redis::AsyncCommands;

pub async fn perform_refresh(
    db: DatabaseConnection,
    valkey: Option<redis::Client>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: RefreshTokenRequest,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    // 1. Hash the provided refresh token via core service
    let refresh_token_hash = token_service::hash_refresh_token(&req.refresh_token);

    // 2. Find session by hash
    let session = session_repository::find_by_hash(&db, &refresh_token_hash)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    // 3. Check if session is revoked
    if session.revoked_at.is_some() {
        return Err(AppError::Unauthorized("Session has been revoked".to_string()));
    }

    // 4. Check expiry against current time
    let now = Utc::now();
    if session.expires_at < now {
        return Err(AppError::Unauthorized("Refresh token has expired. Please login again.".to_string()));
    }

    // 5. Check whitelist in Valkey if provided
    if let Some(vk) = valkey.clone() {
        let mut conn = vk.get_multiplexed_async_connection().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
        let whitelist_key = format!("whitelist:session:{}", session.id);
        let whitelisted_hash: Option<String> = conn.get(&whitelist_key)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Valkey error: {}", e)))?;

        match whitelisted_hash {
            Some(h) if h == refresh_token_hash => {
                // Token matches whitelist
            }
            _ => {
                // Token not in whitelist or mismatch (possible reuse attack)
                // Revoke session for safety
                let _ = session_repository::revoke(&db, session.id).await;
                return Err(AppError::Unauthorized("Invalid session state. Please login again.".to_string()));
            }
        }
    }

    // 6. Generate new tokens (Rotation) via core service
    let user = user_repository::find_by_id(&db, session.user_id)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading user: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User not found for session")))?;

    let token_bundle = token_service::generate_token_bundle(
        &user.id.to_string(),
        access_token_ttl.0,
        &encoding_key,
    )?;

    // 7. Update session in DB (ONLY rotate the hash, do NOT extend expiry)
    let mut session_active: sessions::ActiveModel = session.clone().into_active_model();
    session_active.refresh_token_hash = Set(token_bundle.refresh_token_hash.clone());
    // session_active.expires_at remains unchanged from original login

    session_repository::update(&db, session_active)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update session: {:?}", e)))?;

    // 8. Update whitelist in Valkey with remaining time only
    if let Some(vk) = valkey {
        let mut conn = vk.get_multiplexed_async_connection().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
        let now_ts = Utc::now().timestamp();
        let expires_at_ts = session.expires_at.timestamp();
        let remaining_seconds = if expires_at_ts > now_ts {
            (expires_at_ts - now_ts) as u64
        } else {
            0
        };

        let whitelist_key = format!("whitelist:session:{}", session.id);
        let _: () = conn.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining_seconds)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update whitelist: {}", e)))?;
    }

    Ok(ApiResponse::success(
        200,
        "Token refreshed successfully",
        LoginResponseData {
            user: UserInfo {
                full_name: user.full_name.clone(),
                account_status: AccountStatusEnum::from(user.account_status),
                email: user.email.clone(),
                phone_number: user.phone_number.clone(),
                role_id: user.role_id,
            },
            access_token: token_bundle.access_token,
            refresh_token: token_bundle.refresh_token,
        },
    ))
}
