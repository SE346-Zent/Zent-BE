use sea_orm::*;
use sea_orm::prelude::Expr;
use crate::{
    core::{
        errors::AppError,
        state::AccessTokenDefaultTTLSeconds,
    },
    entities::{sessions, users},
    model::{
        requests::auth::refresh_token_request::RefreshTokenRequest,
        responses::{
            base::ApiResponse,
            auth::login_response::{LoginResponseData, UserInfo, AccountStatusEnum},
        },
    },
    services::v1::core::token_service,
};
use redis::AsyncCommands;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Describes the outcome of a refresh token attempt.
pub enum RefreshTokenIntent {
    Success {
        user_info: UserInfo,
        token_bundle: token_service::TokenBundle,
        session_id: uuid::Uuid,
        whitelist_key: String,
        remaining_ttl: u64,
    },
    ReuseAttack {
        session_id: uuid::Uuid,
    },
    Error(AppError),
}

/// Pure logic to decide the outcome of a refresh token attempt.
pub fn decide_refresh_token(
    session: sessions::Model,
    user: users::Model,
    whitelisted_hash: Option<String>,
    current_hash: String,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> RefreshTokenIntent {
    if session.revoked_at.is_some() || session.expires_at < Utc::now() {
        return RefreshTokenIntent::Error(AppError::Unauthorized("Session invalid or expired".to_string()));
    }

    if whitelisted_hash.as_deref() != Some(&current_hash) {
        return RefreshTokenIntent::ReuseAttack { session_id: session.id };
    }

    let token_bundle = match token_service::generate_token_bundle(&user.id.to_string(), access_token_ttl.0, encoding_key) {
        Ok(t) => t,
        Err(e) => return RefreshTokenIntent::Error(e),
    };

    let remaining = (session.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;

    RefreshTokenIntent::Success {
        user_info: UserInfo {
            full_name: user.full_name.clone(),
            account_status: AccountStatusEnum::from(user.account_status),
            email: user.email.clone(),
            phone_number: user.phone_number.clone(),
            role_id: user.role_id,
        },
        token_bundle,
        session_id: session.id,
        whitelist_key: format!("whitelist:session:{}", session.id),
        remaining_ttl: remaining,
    }
}

pub async fn handle_refresh_token(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: RefreshTokenRequest,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    // 1. Fetch data (I/O)
    let refresh_token_hash = token_service::hash_refresh_token(&req.refresh_token);
    let session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let whitelist_key = format!("whitelist:session:{}", session.id);
    let whitelisted: Option<String> = conn.get(&whitelist_key).await?;

    let user = users::Entity::find_by_id(session.user_id)
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

    // 2. Decision Logic (Pure)
    let intent = decide_refresh_token(session, user, whitelisted, refresh_token_hash.clone(), access_token_ttl, &encoding_key);

    // 3. Execution (I/O)
    match intent {
        RefreshTokenIntent::Success { user_info, token_bundle, session_id, whitelist_key, remaining_ttl } => {
            let rotation_result = sessions::Entity::update_many()
                .col_expr(sessions::Column::RefreshTokenHash, Expr::value(token_bundle.refresh_token_hash.clone()))
                .filter(sessions::Column::Id.eq(session_id))
                .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
                .exec(&db)
                .await?;

            if rotation_result.rows_affected == 0 {
                return Err(AppError::Unauthorized("Rotation failed".to_string()));
            }

            let _: () = conn.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining_ttl).await?;

            Ok(ApiResponse::success(200, "Refreshed", LoginResponseData {
                user: user_info,
                access_token: token_bundle.access_token,
                refresh_token: token_bundle.refresh_token,
            }))
        }
        RefreshTokenIntent::ReuseAttack { session_id } => {
            let _ = sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::Id.eq(session_id))
                .exec(&db)
                .await;
            Err(AppError::Unauthorized("Suspected reuse attack".to_string()))
        }
        RefreshTokenIntent::Error(err) => Err(err),
    }
}
