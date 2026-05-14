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
pub async fn reset_password_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Password reset service temporarily unavailable. Please try again later.".to_string()))?;
    let mut conn = client.get_connection().await?;
    let reset_token_key = format!("password_reset_token:{}", payload.reset_token);
    let email: Option<String> = conn.get(&reset_token_key).await?;
    let email = email.ok_or_else(|| AppError::BadRequest("Invalid or expired token".to_string()))?;

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("User missing".to_string()))?;

    let is_same = hasher::verify_password(payload.new_password.clone(), user.password_hash.clone()).await?;
    let new_hash = hasher::hash_password(payload.new_password).await?;
    let effect = reset_password::decide_reset_password(&user, is_same, new_hash, reset_token_key)?;

    let mut user_active: users::ActiveModel = user.into();
    user_active.password_hash = Set(effect.new_hash);
    user_active.updated_at = Set(Utc::now());
    user_active.update(db.as_ref()).await?;

    let active_sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .all(db.as_ref()).await?;

    sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
        .filter(sessions::Column::UserId.eq(effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(db.as_ref()).await?;

    for session in active_sessions {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        conn.del::<_, ()>(&whitelist_key).await?;
    }
    conn.del::<_, ()>(&effect.reset_token_key).await?;

    Ok(Json(ApiResponse::message_only(200, "Password reset successful")))
}
