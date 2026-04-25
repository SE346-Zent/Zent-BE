use crate::entities::sessions;
use crate::core::errors::AppError;
use crate::model::requests::auth::user_login_request::UserLoginRequest;
use crate::model::responses::auth::login_response::{
    AccountStatusEnum, LoginResponseData, UserInfo,
};
use crate::model::responses::base::ApiResponse;
use crate::repository::{role_repository, session_repository, user_repository};
use crate::services::v1::core::token_service;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use chrono::Utc;
use jsonwebtoken::EncodingKey;
use sea_orm::*;
use uuid::Uuid;

pub async fn perform_login(
    db: DatabaseConnection,
    valkey: Option<redis::Client>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: UserLoginRequest,
    ip_address: String,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    // 1. Lookup user via repository
    let user_model = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // 2. Check account status
    let status = AccountStatusEnum::from(user_model.account_status);
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

    // 2.5. Check Role exists via repository
    let role_exists = role_repository::find_by_id(&db, user_model.role_id)
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

    // 4. Create Access and Refresh Tokens via core service
    let token_bundle = token_service::generate_token_bundle(
        &user_model.id.to_string(),
        access_token_ttl.0,
        &encoding_key,
    )?;

    let now = Utc::now().timestamp();
    let session_ttl_seconds = session_ttl.0;

    // 5. Create session via repository
    let session_id = Uuid::new_v4();
    let expires_at_chrono =
        chrono::DateTime::from_timestamp(now + session_ttl_seconds, 0).unwrap_or(Utc::now());

    let session_model = sessions::ActiveModel {
        id: Set(session_id),
        user_id: Set(user_model.id),
        refresh_token_hash: Set(token_bundle.refresh_token_hash.clone()),
        device_fingerprint: Set(user_model.id.to_string()), // TODO: properly extract when device_fingerprint features are added
        ip_address: Set(ip_address.chars().take(45).collect()),
        created_at: Set(Utc::now()),
        expires_at: Set(expires_at_chrono),
        revoked_at: Set(None),
    };

    session_repository::create(&db, session_model)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create session: {:?}", e)))?;

    // 6. Whitelist the refresh token in Valkey if provided
    if let Some(vk) = valkey {
        let mut conn = vk.get_multiplexed_async_connection().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
        
        let whitelist_key = format!("whitelist:session:{}", session_id);
        let _: () = redis::AsyncCommands::set_ex(&mut conn, &whitelist_key, &token_bundle.refresh_token_hash, session_ttl_seconds as u64)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to whitelist token: {}", e)))?;
    }

    Ok(ApiResponse::success(
        200,
        "Login successful",
        LoginResponseData {
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
    ))
}
