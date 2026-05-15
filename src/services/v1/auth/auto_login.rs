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
use uuid::Uuid;

/// Describes the outcome of an auto-login attempt.
pub enum AutoLoginEffect {
    Success {
        user_info: UserInfo,
        token_bundle: token_service::TokenBundle,
        new_session_id: Uuid,
        old_session_id: Uuid,
        /// Remaining TTL of the *old* session (used to bound the new Valkey entry)
        remaining_ttl: u64,
    },
    ReuseAttack {
        session_id: Uuid,
    },
}

/// Pure logic: decide the outcome of an auto-login attempt.
///
/// Auto-login differs from refresh-token in that it creates a brand-new session
/// rather than rotating the token in-place. The old session is revoked by the
/// handler so the cron job can clean it up later.
pub fn decide_auto_login(
    session: &sessions::Model,
    user: &users::Model,
    whitelisted_hash: Option<String>,
    current_hash: &str,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<AutoLoginEffect, AppError> {
    // 1. Session must not be revoked or expired
    if session.revoked_at.is_some() || session.expires_at < Utc::now() {
        return Err(AppError::Unauthorized("Session invalid or expired".to_string()));
    }

    // 2. Valkey whitelist must match — otherwise it's a reuse attack
    if whitelisted_hash.as_deref() != Some(current_hash) {
        return Ok(AutoLoginEffect::ReuseAttack {
            session_id: session.id,
        });
    }

    // 3. User must be active
    let status = AccountStatusEnum::from(user.account_status);
    if status != AccountStatusEnum::Active {
        return Err(AppError::Forbidden(format!(
            "Account is {:?}",
            status
        )));
    }

    // 4. Generate fresh token bundle for the new session
    let token_bundle = token_service::generate_token_bundle(
        &user.id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    let remaining = (session.expires_at.timestamp() - Utc::now().timestamp()).max(0) as u64;

    Ok(AutoLoginEffect::Success {
        user_info: UserInfo {
            full_name: user.full_name.clone(),
            account_status: status,
            email: user.email.clone(),
            phone_number: user.phone_number.clone(),
            role_id: user.role_id,
        },
        token_bundle,
        new_session_id: Uuid::new_v4(),
        old_session_id: session.id,
        remaining_ttl: remaining,
    })
}
