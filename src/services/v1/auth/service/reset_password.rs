use sea_orm::*;
use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::reset_password_request::ResetPasswordRequest,
        responses::base::ApiResponse,
    },
    repository::{user_repository, session_repository},
};
use crate::utils::hasher;
use redis::AsyncCommands;
use chrono::Utc;

pub async fn handle_reset_password(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    req: ResetPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let reset_token_key = format!("password_reset_token:{}", req.reset_token);
    let email: Option<String> = conn.get(&reset_token_key).await?;
    let email = email.ok_or_else(|| AppError::BadRequest("Invalid or expired token".to_string()))?;

    let user = user_repository::find_by_email(&db, &email).await?
        .ok_or_else(|| AppError::NotFound("User missing".to_string()))?;

    // Same password check
    let is_same = hasher::verify_password(req.new_password.clone(), user.password_hash.clone()).await?;
    if is_same {
        return Err(AppError::BadRequest("New password cannot be the same as current".to_string()));
    }

    let new_hash = hasher::hash_password(req.new_password).await?;
    let user_id = user.id;
    let mut user_active: users::ActiveModel = user.into();
    user_active.password_hash = Set(new_hash);
    user_active.updated_at = Set(Utc::now());
    user_repository::update(&db, user_active).await?;

    // Revoke sessions
    let active_sessions = crate::entities::sessions::Entity::find()
        .filter(crate::entities::sessions::Column::UserId.eq(user_id))
        .filter(crate::entities::sessions::Column::RevokedAt.is_null())
        .all(&db).await?;

    let _ = session_repository::revoke_all_by_user_id(&db, user_id).await;

    for session in active_sessions {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
    }

    let _: () = conn.del(&reset_token_key).await?;

    Ok(ApiResponse::message_only(200, "Password reset successful"))
}
