use crate::{
    core::{
        errors::AppError,
        state::AccessTokenDefaultTTLSeconds,
    },
    entities::{sessions, users},
    model::responses::auth::login_response::{UserInfo, AccountStatusEnum},
    services::v1::core::token_service,
};
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Describes the outcome of a refresh token attempt.
pub enum RefreshTokenEffect {
    Success {
        user_info: UserInfo,
        token_bundle: token_service::TokenBundle,
        session_id: uuid::Uuid,
        remaining_ttl: u64,
    },
    ReuseAttack {
        session_id: uuid::Uuid,
    },
}

/// Pure logic to decide the outcome of a refresh token attempt.
pub fn decide_refresh_token(
    session: &sessions::Model,
    user: &users::Model,
    whitelisted_hash: Option<String>,
    current_hash: &str,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<RefreshTokenEffect, AppError> {
    if session.revoked_at.is_some() || session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Session invalid or expired".to_string()));
    }

    if whitelisted_hash.as_deref() != Some(current_hash) {
        return Ok(RefreshTokenEffect::ReuseAttack { session_id: session.id });
    }

    let token_bundle = token_service::generate_token_bundle(&user.id.to_string(), access_token_ttl.0, encoding_key)?;

    let remaining = (session.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;

    Ok(RefreshTokenEffect::Success {
        user_info: UserInfo {
            full_name: user.full_name.clone(),
            account_status: AccountStatusEnum::from(user.account_status),
            email: user.email.clone(),
            phone_number: user.phone_number.clone(),
            role_id: user.role_id,
        },
        token_bundle,
        session_id: session.id,
        remaining_ttl: remaining,
    })
}
