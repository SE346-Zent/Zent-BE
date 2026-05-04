use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::users,
    model::responses::auth::login_response::{LoginResponseData, UserInfo, AccountStatusEnum},
    services::v1::core::token_service,
};

use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

/// Plain struct representing the side-effects that need to be persisted
pub struct LoginEffect {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub refresh_token_hash: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub response_data: LoginResponseData,
}

/// Pure logic to decide the outcome of a login attempt.
pub fn decide_login(
    user_model: &users::Model,
    is_password_valid: bool,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: &EncodingKey,
) -> Result<LoginEffect, AppError> {
    // 1. Check if user is deleted
    if user_model.deleted_at.is_some() {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 2. Verify password (passed in)
    if !is_password_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 3. Verify account status
    let status = AccountStatusEnum::from(user_model.account_status);
    match status {
        AccountStatusEnum::Active => {} 
        AccountStatusEnum::Pending => {
            return Err(AppError::Forbidden("Account is pending verification".to_string()));
        }
        _ => {
            return Err(AppError::Forbidden(format!("Account is {:?}", status)));
        }
    }

    // 4. Generate tokens
    let token_bundle = token_service::generate_token_bundle(
        &user_model.id.to_string(),
        access_token_ttl.0,
        encoding_key,
    )?;

    // 5. Prepare session data
    let session_id = Uuid::new_v4();
    let session_ttl_seconds = session_ttl.0;
    let expires_at = Utc::now() + chrono::Duration::seconds(session_ttl_seconds as i64);

    Ok(LoginEffect {
        session_id,
        user_id: user_model.id,
        refresh_token_hash: token_bundle.refresh_token_hash,
        expires_at,
        response_data: LoginResponseData {
            user: UserInfo {
                full_name: user_model.full_name.clone(),
                account_status: status,
                email: user_model.email.clone(),
                phone_number: user_model.phone_number.clone(),
                role_id: user_model.role_id,
            },
            access_token: token_bundle.access_token,
            refresh_token: token_bundle.refresh_token,
        },
    })
}
