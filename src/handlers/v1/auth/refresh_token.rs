use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, prelude::Expr};
use validator::Validate;
use jsonwebtoken::EncodingKey;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::AccessTokenDefaultTTLSeconds;
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::{users, sessions};
use crate::services::v1::auth::refresh_token;
use crate::services::v1::core::token_service;
use crate::model::requests::auth::refresh_token_request::RefreshTokenRequest;
use crate::model::responses::base::ApiResponse;
use redis::AsyncCommands;

use crate::model::responses::auth::login_response::LoginResponseData;

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh-token",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
/// Handle requests to refresh an access token using a valid refresh token.
///
/// This handler verifies the provided refresh token, checks it against the
/// whitelist cache to detect reuse attacks, rotates the refresh token in the
/// database (MySQL) and cache (Valkey), and returns a new token pair.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `valkey_client` - Optional shared Valkey client for session whitelisting.
/// * `access_token_ttl` - Default duration for the new access token.
/// * `encoding_key` - Key for signing the new JWTs.
/// * `refresh_token_payload` - The request containing the existing refresh token.
///
/// # Returns
/// A result containing the successful `ApiResponse` with new login data, or an `AppError`.
pub async fn refresh_token_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    Json(refresh_token_payload): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    refresh_token_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let refresh_token_hash = token_service::hash_refresh_token(&refresh_token_payload.refresh_token);
    let session_record = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Session service temporarily unavailable. Please try again later.".to_string()))?;
    let mut valkey_conn = client.get_connection().await?;
    let whitelist_key = format!("whitelist:session:{}", session_record.id);
    let whitelisted_token_hash: Option<String> = valkey_conn.get(&whitelist_key).await?;

    let user_record = users::Entity::find_by_id(session_record.user_id)
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

    let refresh_effect = refresh_token::decide_refresh_token(&session_record, &user_record, whitelisted_token_hash, &refresh_token_hash, access_token_ttl, &encoding_key)?;

    match refresh_effect {
        refresh_token::RefreshTokenEffect::Success { user_info, token_bundle, session_id, remaining_session_ttl } => {
            let rotation_result = sessions::Entity::update_many()
                .col_expr(sessions::Column::RefreshTokenHash, Expr::value(token_bundle.refresh_token_hash.clone()))
                .filter(sessions::Column::Id.eq(session_id))
                .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
                .exec(db_connection.as_ref()).await?;

            if rotation_result.rows_affected == 0 {
                return Err(AppError::Unauthorized("Rotation failed".to_string()));
            }
            let _: () = valkey_conn.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining_session_ttl).await?;
            Ok(Json(ApiResponse::success(200, "Refreshed", LoginResponseData {
                user: user_info, access_token: token_bundle.access_token, refresh_token: token_bundle.refresh_token,
            })))
        }
        refresh_token::RefreshTokenEffect::ReuseAttackDetected { session_id } => {
            sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::Id.eq(session_id))
                .exec(db_connection.as_ref()).await?;
            Err(AppError::Unauthorized("Suspected reuse attack".to_string()))
        }
    }
}
