use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::sessions;
use crate::extractor::auth_user::AuthUser;
use crate::services::v1::auth::logout;
use crate::services::v1::core::token_service;
use redis::AsyncCommands;

use crate::model::requests::auth::logout_request::LogoutRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn logout_handler(
    auth: AuthUser,
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let refresh_token_hash = token_service::hash_refresh_token(&payload.refresh_token);
    let session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    let effect = logout::decide_logout(&session, auth.user.id)?;

    let mut session_active: sessions::ActiveModel = session.into();
    session_active.revoked_at = Set(Some(Utc::now()));
    session_active.update(db.as_ref()).await?;

    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", effect.session_id);
            let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
        } else {
            tracing::warn!("Valkey unavailable — session whitelist not cleared");
        }
    }

    Ok(Json(ApiResponse::message_only(200, "Logout successful")))
}
