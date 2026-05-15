use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, prelude::Expr, ActiveModelTrait, Set};
use validator::Validate;
use jsonwebtoken::EncodingKey;
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::{users, sessions};
use crate::services::v1::auth::auto_login;
use crate::services::v1::core::token_service;
use crate::model::requests::auth::auto_login_request::AutoLoginRequest;
use crate::model::responses::base::ApiResponse;
use redis::AsyncCommands;

use crate::model::responses::auth::login_response::LoginResponseData;

#[utoipa::path(
    post,
    path = "/api/v1/auth/auto-login",
    request_body = AutoLoginRequest,
    responses(
        (status = 200, description = "Auto-login successful", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn auto_login_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(session_ttl): State<SessionDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    Json(payload): Json<AutoLoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let refresh_token_hash = token_service::hash_refresh_token(&payload.refresh_token);

    // Look up the session by refresh token hash
    let old_session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    // Check Valkey whitelist TTL (fresh connection)
    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Session service temporarily unavailable. Please try again later.".to_string()))?;
    let whitelist_key = format!("whitelist:session:{}", old_session.id);
    let whitelisted: Option<String> = {
        let mut conn = client.get_connection().await?;
        conn.get(&whitelist_key).await?
    };

    // Load the user
    let user = users::Entity::find_by_id(old_session.user_id)
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

    // Call pure service logic
    let effect = auto_login::decide_auto_login(
        &old_session,
        &user,
        whitelisted,
        &refresh_token_hash,
        access_token_ttl,
        &encoding_key,
    )?;

    match effect {
        auto_login::AutoLoginEffect::Success { user_info, token_bundle, new_session_id, old_session_id, remaining_ttl } => {
            // Create the new session
            let new_session_expires_at = Utc::now() + chrono::Duration::seconds(session_ttl.0 as i64);
            let new_session = sessions::ActiveModel {
                id: Set(new_session_id),
                user_id: Set(user.id),
                refresh_token_hash: Set(token_bundle.refresh_token_hash.clone()),
                ip_address: Set(old_session.ip_address.clone()),
                device_fingerprint: Set(old_session.device_fingerprint.clone()),
                created_at: Set(Utc::now()),
                expires_at: Set(new_session_expires_at),
                ..Default::default()
            };
            new_session.insert(db.as_ref()).await?;

            // Revoke the old session (cron will clean it up)
            sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(Utc::now()))
                .filter(sessions::Column::Id.eq(old_session_id))
                .exec(db.as_ref()).await?;

            // Set Valkey whitelist for the new session (fresh connection)
            let ttl = remaining_ttl.min(session_ttl.0 as u64);
            let new_whitelist_key = format!("whitelist:session:{}", new_session_id);
            {
                let mut conn2 = client.get_connection().await?;
                let _: () = conn2.set_ex(&new_whitelist_key, &token_bundle.refresh_token_hash, ttl).await?;
            }

            Ok(Json(ApiResponse::success(200, "Auto-login successful", LoginResponseData {
                user: user_info,
                access_token: token_bundle.access_token,
                refresh_token: token_bundle.refresh_token,
            })))
        }
        auto_login::AutoLoginEffect::ReuseAttack { session_id } => {
            // Revoke the session on suspected reuse attack
            sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(Utc::now()))
                .filter(sessions::Column::Id.eq(session_id))
                .exec(db.as_ref()).await?;
            Err(AppError::Unauthorized("Suspected reuse attack".to_string()))
        }
    }
}
