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
pub async fn refresh_token_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let refresh_token_hash = token_service::hash_refresh_token(&payload.refresh_token);
    let session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Session service temporarily unavailable. Please try again later.".to_string()))?;
    let whitelist_key = format!("whitelist:session:{}", session.id);
    let whitelisted: Option<String> = {
        let mut conn = client.get_connection().await?;
        conn.get(&whitelist_key).await?
    };

    let user = users::Entity::find_by_id(session.user_id)
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

    let effect = refresh_token::decide_refresh_token(&session, &user, whitelisted, &refresh_token_hash, access_token_ttl, &encoding_key)?;

    match effect {
        refresh_token::RefreshTokenEffect::Success { user_info, token_bundle, session_id, remaining_ttl } => {
            let rotation_result = sessions::Entity::update_many()
                .col_expr(sessions::Column::RefreshTokenHash, Expr::value(token_bundle.refresh_token_hash.clone()))
                .filter(sessions::Column::Id.eq(session_id))
                .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
                .exec(db.as_ref()).await?;

            if rotation_result.rows_affected == 0 {
                return Err(AppError::Unauthorized("Rotation failed".to_string()));
            }
            {
                let mut conn2 = client.get_connection().await?;
                let _: () = conn2.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining_ttl).await?;
            }
            Ok(Json(ApiResponse::success(200, "Refreshed", LoginResponseData {
                user: user_info, access_token: token_bundle.access_token, refresh_token: token_bundle.refresh_token,
            })))
        }
        refresh_token::RefreshTokenEffect::ReuseAttack { session_id } => {
            sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::Id.eq(session_id))
                .exec(db.as_ref()).await?;
            Err(AppError::Unauthorized("Suspected reuse attack".to_string()))
        }
    }
}
