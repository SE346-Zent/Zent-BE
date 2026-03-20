use crate::entities::{session, user};
use crate::model::auth::jwt_claims::Claims;
use crate::model::requests::auth::user_login_request::UserLoginRequest;
use crate::model::responses::auth::login_response::{
    AccountStatusEnum, LoginResponse, LoginResponseData,
};
use crate::model::responses::error::AppError;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
// use axum::extract::ConnectInfo;
use crate::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use rand::RngCore;
use sea_orm::*;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use uuid::Uuid;

pub async fn perform_login(
    db: DatabaseConnection,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: UserLoginRequest,
    ip_addr: SocketAddr,
) -> Result<LoginResponse, AppError> {
    // 1. Lookup user
    let user_opt = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let user_model = match user_opt {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Invalid credentials".to_string())),
    };

    // 2. Check account status
    let status = AccountStatusEnum::from_i32(user_model.account_status);
    match status {
        AccountStatusEnum::Active => {}
        AccountStatusEnum::Pending => {
            return Err(AppError::Forbidden(
                "Email address not verified".to_string(),
            ));
        }
        AccountStatusEnum::Terminated | AccountStatusEnum::Inactive | AccountStatusEnum::Locked => {
            return Err(AppError::Forbidden("Account not available".to_string()));
        }
        _ => return Err(AppError::Forbidden("Account state unknown".to_string())),
    };

    // 2.5. Check Role exists
    let role_exists = crate::entities::role::Entity::find_by_id(user_model.role_id)
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if role_exists.is_none() {
        return Err(AppError::Forbidden("Account state unknown".to_string()));
    }

    // 3. Validate password
    let password_hash_str = user_model.password_hash.clone();
    let password_bytes = req.password.into_bytes();
    let is_valid = tokio::task::spawn_blocking(move || {
        let parsed_hash = match PasswordHash::new(&password_hash_str) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(&password_bytes, &parsed_hash)
            .is_ok()
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task")))?;

    if !is_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 4. Create Access and Refresh Tokens
    let now = Utc::now().timestamp();
    let access_token_ttl_seconds = access_token_ttl.0;
    let session_ttl_seconds = session_ttl.0;

    let claims = Claims {
        sub: user_model.id.to_string(),
        iat: now as usize,
        exp: (now + access_token_ttl_seconds) as usize,
    };

    let access_token =
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to encode token: {}", e)))?;

    // Generate refresh token
    let mut refresh_token_bytes = [0u8; 48];
    rand::rng().fill_bytes(&mut refresh_token_bytes);
    let refresh_token = URL_SAFE.encode(refresh_token_bytes);

    // Hash refresh token for storage
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let refresh_token_hash = format!("{:x}", hasher.finalize());

    // 5. Create session
    let session_id = Uuid::new_v4();
    let expires_at_chrono =
        chrono::DateTime::from_timestamp(now + session_ttl_seconds, 0).unwrap_or(Utc::now());
    let _ = session::ActiveModel {
        id: Set(session_id),
        user_id: Set(user_model.id),
        refresh_token_hash: Set(refresh_token_hash),
        device_fingerprint: Set(user_model.id.to_string()), // TODO: properly extract when device_fingerprint features are added
        ip_address: Set(ip_addr.ip().to_string().chars().take(45).collect()),
        created_at: Set(Utc::now()),
        expires_at: Set(expires_at_chrono),
        revoked_at: Set(None),
    }
    .insert(&db)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create session: {:?}", e)))?;

    Ok(LoginResponse::success(LoginResponseData {
        account_status: status,
        email: user_model.email.clone(),
        phone: user_model.phone_number.clone(),
        role_id: user_model.role_id,
        access_token,
        refresh_token,
    }))
}
