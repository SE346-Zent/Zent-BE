use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set, prelude::Expr};
use validator::Validate;
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::{users, sessions};
use crate::utils::hasher;
use crate::services::v1::auth::reset_password;
use crate::model::requests::auth::reset_password_request::ResetPasswordRequest;
use redis::AsyncCommands;

use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successfully", body = MessageOnlyResponse),
        (status = 400, description = "Invalid token", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
/// Handle requests to reset a user's password using a valid reset token.
///
/// This handler validates the reset token from Valkey, verifies that the new
/// password is not the same as the current one, updates the password hash in
/// MySQL, revokes all active sessions for the user, and clears the session
/// whitelist cache.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `valkey_client` - Optional shared Valkey client for token validation and cache clearing.
/// * `reset_payload` - The request containing the reset token and new password.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn reset_password_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(reset_payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    reset_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Password reset service temporarily unavailable. Please try again later.".to_string()))?;
    let mut valkey_conn = client.get_connection().await?;
    let reset_token_cache_key = format!("password_reset_token:{}", reset_payload.reset_token);
    let user_email: Option<String> = valkey_conn.get(&reset_token_cache_key).await?;
    let user_email = user_email.ok_or_else(|| AppError::BadRequest("Invalid or expired token".to_string()))?;

    let user_record = users::Entity::find()
        .filter(users::Column::Email.eq(&user_email))
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("User missing".to_string()))?;

    let is_same_password = hasher::verify_password(reset_payload.new_password.clone(), user_record.password_hash.clone()).await?;
    let new_password_hash = hasher::hash_password(reset_payload.new_password).await?;
    let reset_effect = reset_password::decide_reset_password(&user_record, is_same_password, new_password_hash, reset_token_cache_key)?;

    let mut user_active_model: users::ActiveModel = user_record.into();
    user_active_model.password_hash = Set(reset_effect.new_password_hash);
    user_active_model.updated_at = Set(Utc::now());
    user_active_model.update(db_connection.as_ref()).await?;

    let active_sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(reset_effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .all(db_connection.as_ref()).await?;

    sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
        .filter(sessions::Column::UserId.eq(reset_effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(db_connection.as_ref()).await?;

    for session in active_sessions {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        valkey_conn.del::<_, ()>(&whitelist_key).await?;
    }
    valkey_conn.del::<_, ()>(&reset_effect.reset_token_cache_key).await?;

    Ok(Json(ApiResponse::message_only(200, "Password reset successful")))
}
