use sea_orm::*;
use crate::{
    core::{
        errors::AppError,
        state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    entities::sessions,
    model::{
        requests::auth::user_login_request::UserLoginRequest,
        responses::{
            base::ApiResponse,
            auth::login_response::{LoginResponseData, UserInfo, AccountStatusEnum},
        },
    },
    repository::user_repository,
    services::v1::core::token_service,
    utils::hasher,
};

use uuid::Uuid;
use chrono::Utc;
use jsonwebtoken::EncodingKey;

pub async fn handle_login(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
    req: UserLoginRequest,
    ip_address: String,
) -> Result<ApiResponse<LoginResponseData>, AppError> {
    // 1. Find user by email
    let user_model = user_repository::find_by_email(&db, &req.email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // 2. Check if user is deleted
    if user_model.deleted_at.is_some() {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 3. Verify password
    let is_valid = hasher::verify_password(req.password, user_model.password_hash.clone()).await?;
    if !is_valid {
        return Err(AppError::Unauthorized("Invalid credentials".to_string()));
    }

    // 4. Verify account status
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

    // 5. Generate tokens
    let token_bundle = token_service::generate_token_bundle(
        &user_model.id.to_string(),
        access_token_ttl.0,
        &encoding_key,
    )?;

    // 6. Create session in DB
    let session_id = Uuid::new_v4();
    let session_ttl_seconds = session_ttl.0;
    let expires_at = Utc::now() + chrono::Duration::seconds(session_ttl_seconds);

    let active_session = sessions::ActiveModel {
        id: Set(session_id),
        user_id: Set(user_model.id),
        refresh_token_hash: Set(token_bundle.refresh_token_hash.clone()),
        ip_address: Set(ip_address),
        device_fingerprint: Set(user_model.id.to_string()),
        expires_at: Set(expires_at),
        ..Default::default()
    };

    active_session.insert(&db).await?;

    // 7. Whitelist in Valkey if provided
    if let Some(mut conn) = valkey {
        let whitelist_key = format!("whitelist:session:{}", session_id);
        let _: () = redis::AsyncCommands::set_ex(&mut conn, &whitelist_key, &token_bundle.refresh_token_hash, session_ttl_seconds as u64)
            .await?;
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
