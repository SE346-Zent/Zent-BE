use sea_orm::*;
use sea_orm::prelude::Expr;
use crate::{
    core::errors::AppError,
    entities::{users, sessions},
    model::{
        requests::auth::reset_password_request::ResetPasswordRequest,
        responses::base::ApiResponse,
    },
};
use crate::utils::hasher;
use redis::AsyncCommands;
use chrono::Utc;

/// Describes the outcome of a password reset attempt.
pub enum ResetPasswordIntent {
    Success {
        user_active: users::ActiveModel,
        user_id: uuid::Uuid,
        reset_token_key: String,
    },
    Error(AppError),
}

/// Pure logic to decide the outcome of a password reset attempt.
pub fn decide_reset_password(
    user_model: users::Model,
    is_same_password: bool,
    new_hash: String,
    reset_token_key: String,
) -> ResetPasswordIntent {
    if is_same_password {
        return ResetPasswordIntent::Error(AppError::BadRequest("New password cannot be the same as current".to_string()));
    }

    let user_id = user_model.id;
    let mut user_active: users::ActiveModel = user_model.into();
    user_active.password_hash = Set(new_hash);
    user_active.updated_at = Set(Utc::now());

    ResetPasswordIntent::Success {
        user_active,
        user_id,
        reset_token_key,
    }
}

pub async fn handle_reset_password(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    req: ResetPasswordRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Fetch data (I/O)
    let mut conn = valkey.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let reset_token_key = format!("password_reset_token:{}", req.reset_token);
    let email: Option<String> = conn.get(&reset_token_key).await?;
    let email = email.ok_or_else(|| AppError::BadRequest("Invalid or expired token".to_string()))?;

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&db)
        .await?
        .ok_or_else(|| AppError::NotFound("User missing".to_string()))?;

    // 2. Async logic (Password hashing)
    let is_same = hasher::verify_password(req.new_password.clone(), user.password_hash.clone()).await?;
    let new_hash = hasher::hash_password(req.new_password).await?;

    // 3. Decision Logic (Pure)
    let intent = decide_reset_password(user, is_same, new_hash, reset_token_key);

    // 4. Execution (I/O)
    match intent {
        ResetPasswordIntent::Success { user_active, user_id, reset_token_key } => {
            user_active.update(&db).await?;

            // Revoke sessions (I/O)
            let active_sessions = sessions::Entity::find()
                .filter(sessions::Column::UserId.eq(user_id))
                .filter(sessions::Column::RevokedAt.is_null())
                .all(&db).await?;

            let _ = sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::UserId.eq(user_id))
                .filter(sessions::Column::RevokedAt.is_null())
                .exec(&db)
                .await;

            for session in active_sessions {
                let whitelist_key = format!("whitelist:session:{}", session.id);
                let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
            }

            let _: () = conn.del(&reset_token_key).await?;

            Ok(ApiResponse::message_only(200, "Password reset successful"))
        }
        ResetPasswordIntent::Error(err) => Err(err),
    }
}
