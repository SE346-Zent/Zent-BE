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

pub async fn handle_refresh_token(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: RefreshTokenRequest,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    let refresh_token_hash = token_service::hash_refresh_token(&req.refresh_token);
    let session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    if session.revoked_at.is_some() || session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Session invalid or expired".to_string()));
    }

    if let Some(mut conn) = valkey {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        let whitelisted: Option<String> = conn.get(&whitelist_key).await?;
        if whitelisted.as_deref() != Some(&refresh_token_hash) {
            let _ = sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::Id.eq(session.id))
                .exec(&db)
                .await;
            return Err(AppError::Unauthorized("Suspected reuse attack".to_string()));
        }

        let user = users::Entity::find_by_id(session.user_id)
            .one(&db)
            .await?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

        let token_bundle = token_service::generate_token_bundle(&user.id.to_string(), access_token_ttl.0, &encoding_key)?;

        let rotation_result = sessions::Entity::update_many()
            .col_expr(sessions::Column::RefreshTokenHash, Expr::value(token_bundle.refresh_token_hash.clone()))
            .filter(sessions::Column::Id.eq(session.id))
            .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
            .exec(&db)
            .await?;

        if rotation_result.rows_affected == 0 {
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
