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
/// Handle user logout requests by invalidating the specified session.
///
/// This handler verifies session ownership, marks the session as revoked in
/// the relational database (MySQL), and removes the session from the whitelist
/// cache (Valkey).
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user extracted from the request.
/// * `db_connection` - Shared database connection pool.
/// * `valkey_client` - Optional shared Valkey client for session management.
/// * `logout_payload` - The logout request containing the refresh token.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn logout_handler(
    authenticated_user: AuthUser,
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(logout_payload): Json<LogoutRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let refresh_token_hash = token_service::hash_refresh_token(&logout_payload.refresh_token);
    let session_record = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

    let logout_effect = logout::decide_logout(&session_record, authenticated_user.user.id)?;

    let mut session_active_model: sessions::ActiveModel = session_record.into();
    session_active_model.revoked_at = Set(Some(Utc::now()));
    session_active_model.update(db_connection.as_ref()).await?;

    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", logout_effect.revoked_session_id);
            let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
        } else {
            tracing::warn!("Valkey unavailable — session whitelist not cleared");
        }
    }

    Ok(Json(ApiResponse::message_only(200, "Logout successful")))
}
