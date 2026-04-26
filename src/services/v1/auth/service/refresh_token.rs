use sea_orm::*;
use crate::{
    core::{
        errors::AppError,
        state::AccessTokenDefaultTTLSeconds,
    },
    model::{
        requests::auth::refresh_token_request::RefreshTokenRequest,
        responses::{
            base::ApiResponse,
            auth::login_response::{LoginResponseData, UserInfo, AccountStatusEnum},
        },
    },
    repository::{user_repository, session_repository},
    services::v1::core::token_service,
};
use redis::AsyncCommands;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

pub async fn handle_refresh_token(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: RefreshTokenRequest,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    let refresh_token_hash = token_service::hash_refresh_token(&req.refresh_token);
    let session = session_repository::find_by_hash(&db, &refresh_token_hash).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    if session.revoked_at.is_some() || session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Session invalid or expired".to_string()));
    }

    if let Some(mut conn) = valkey {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        let whitelisted: Option<String> = conn.get(&whitelist_key).await?;
        if whitelisted.as_deref() != Some(&refresh_token_hash) {
            let _ = session_repository::revoke(&db, session.id).await;
            return Err(AppError::Unauthorized("Suspected reuse attack".to_string()));
        }

        let user = user_repository::find_by_id(&db, session.user_id).await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

        let token_bundle = token_service::generate_token_bundle(&user.id.to_string(), access_token_ttl.0, &encoding_key)?;

        if !session_repository::atomic_rotate(&db, session.id, &refresh_token_hash, &token_bundle.refresh_token_hash).await? {
            return Err(AppError::Unauthorized("Rotation failed".to_string()));
        }

        let remaining = (session.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;
        let _: () = conn.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining).await?;

        Ok(ApiResponse::success(200, "Refreshed", LoginResponseData {
            user: UserInfo {
                full_name: user.full_name.clone(),
                account_status: AccountStatusEnum::from(user.account_status),
                email: user.email.clone(),
                phone_number: user.phone_number.clone(),
                role_id: user.role_id,
            },
            access_token: token_bundle.access_token,
            refresh_token: token_bundle.refresh_token,
        }))
    } else {
        Err(AppError::Internal(anyhow::anyhow!("Valkey missing")))
    }
}
