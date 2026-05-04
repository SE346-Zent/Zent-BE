use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    Json, Router, routing::post,
};
use std::net::SocketAddr;
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use sea_orm::prelude::Expr;
use jsonwebtoken::EncodingKey;
use redis::AsyncCommands;
use validator::Validate;
use chrono::Utc;
use crate::{
    core::{
        errors::{AppError, ErrorResponse},
        state::{AppState, AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds},
    },
    infrastructure::cache::ValkeyClient,
    entities::{users, sessions, account_status, roles},
    utils::hasher,
    services::v1::auth::{
        forgot_password, verify_forgot_password_otp, reset_password, 
        refresh_token, login, register, verify_otp, resend_otp,
    },
    services::v1::core::{email_service, token_service},
    model::{
        requests::auth::{
            user_login_request::UserLoginRequest,
            user_registration_request::UserRegistrationRequest,
            verify_otp_request::VerifyOtpRequest,
            resend_otp_request::ResendOtpRequest,
            forgot_password_request::ForgotPasswordRequest,
            verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
            reset_password_request::ResetPasswordRequest,
            refresh_token_request::RefreshTokenRequest,
        },
        responses::{
            auth::login_response::LoginResponseData,
            auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
            base::{ApiResponse, MessageOnlyResponse},
        },
    },
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "OTP sent successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn forgot_password_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. Fetch data (I/O)
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?;

    // 2. Decision Logic (Pure)
    let effect = forgot_password::decide_forgot_password(user.as_ref(), payload)?;

    // 3. Execution (I/O)
    if let Some(client) = valkey_client {
        let mut conn = client.get_connection();
        let valkey_key = format!("forgot_password_verification:{}", effect.email);
        let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_forgot_password_email(&rmq, &templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "OTP sent")))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-forgot-password-otp",
    request_body = VerifyForgotPasswordOtpRequest,
    responses(
        (status = 200, description = "OTP verified successfully", body = ApiResponse<VerifyForgotPasswordOtpResponseData>),
        (status = 400, description = "Invalid OTP", body = ErrorResponse),
        (status = 403, description = "Too many failed attempts", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn verify_forgot_password_otp_handler(
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Json(payload): Json<VerifyForgotPasswordOtpRequest>,
) -> Result<Json<ApiResponse<VerifyForgotPasswordOtpResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. I/O: Interact with Valkey
    let client = valkey_client.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let mut conn = client.get_connection();
    let script_hashes = client.get_script_hashes();
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("forgot_password_verification:{}", payload.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash)
        .arg(1)
        .arg(&valkey_key)
        .arg(&payload.otp_code)
        .query_async(&mut conn)
        .await?;

    // 2. Decision Logic (Pure)
    let effect = verify_forgot_password_otp::decide_verify_forgot_password_otp(result, payload.email)?;

    // 3. Execution (I/O)
    conn.set_ex::<_, _, ()>(&effect.reset_token_key, &effect.email, effect.ttl_seconds).await?;
    
    Ok(Json(ApiResponse::success(200, "Verified", VerifyForgotPasswordOtpResponseData { reset_token: effect.reset_token })))
}

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
    
    // 1. Fetch data (I/O)
    let client = valkey_client.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let mut conn = client.get_connection();
    let reset_token_key = format!("password_reset_token:{}", payload.reset_token);
    let email: Option<String> = conn.get(&reset_token_key).await?;
    let email = email.ok_or_else(|| AppError::BadRequest("Invalid or expired token".to_string()))?;

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("User missing".to_string()))?;

    // 2. Async logic (Password hashing)
    let is_same = hasher::verify_password(payload.new_password.clone(), user.password_hash.clone()).await?;
    let new_hash = hasher::hash_password(payload.new_password).await?;

    // 3. Decision Logic (Pure)
    let effect = reset_password::decide_reset_password(&user, is_same, new_hash, reset_token_key)?;

    // 4. Execution (I/O)
    let mut user_active: users::ActiveModel = user.into();
    user_active.password_hash = Set(effect.new_hash);
    user_active.updated_at = Set(Utc::now());
    user_active.update(db.as_ref()).await?;

    // Revoke sessions (I/O)
    let active_sessions = sessions::Entity::find()
        .filter(sessions::Column::UserId.eq(effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .all(db.as_ref()).await?;

    let _ = sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
        .filter(sessions::Column::UserId.eq(effect.user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .exec(db.as_ref())
        .await;

    for session in active_sessions {
        let whitelist_key = format!("whitelist:session:{}", session.id);
        let _: () = conn.del(&whitelist_key).await.unwrap_or_default();
    }

    let _: () = conn.del(&effect.reset_token_key).await?;

    Ok(Json(ApiResponse::message_only(200, "Password reset successful")))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh-token",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn refresh_token_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. Fetch data (I/O)
    let refresh_token_hash = token_service::hash_refresh_token(&payload.refresh_token);
    let session = sessions::Entity::find()
        .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid token".to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let mut conn = client.get_connection();
    let whitelist_key = format!("whitelist:session:{}", session.id);
    let whitelisted: Option<String> = conn.get(&whitelist_key).await?;

    let user = users::Entity::find_by_id(session.user_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("User missing")))?;

    // 2. Decision Logic (Pure)
    let effect = refresh_token::decide_refresh_token(&session, &user, whitelisted, &refresh_token_hash, access_token_ttl, &encoding_key)?;

    // 3. Execution (I/O)
    match effect {
        refresh_token::RefreshTokenEffect::Success { user_info, token_bundle, session_id, remaining_ttl } => {
            let rotation_result = sessions::Entity::update_many()
                .col_expr(sessions::Column::RefreshTokenHash, Expr::value(token_bundle.refresh_token_hash.clone()))
                .filter(sessions::Column::Id.eq(session_id))
                .filter(sessions::Column::RefreshTokenHash.eq(&refresh_token_hash))
                .exec(db.as_ref())
                .await?;

            if rotation_result.rows_affected == 0 {
                return Err(AppError::Unauthorized("Rotation failed".to_string()));
            }

            let _: () = conn.set_ex(&whitelist_key, &token_bundle.refresh_token_hash, remaining_ttl).await?;

            Ok(Json(ApiResponse::success(200, "Refreshed", LoginResponseData {
                user: user_info,
                access_token: token_bundle.access_token,
                refresh_token: token_bundle.refresh_token,
            })))
        }
        refresh_token::RefreshTokenEffect::ReuseAttack { session_id } => {
            let _ = sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::value(chrono::Utc::now()))
                .filter(sessions::Column::Id.eq(session_id))
                .exec(db.as_ref())
                .await;
            Err(AppError::Unauthorized("Suspected reuse attack".to_string()))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = UserLoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn login_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(session_ttl): State<SessionDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<UserLoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let ip_address = headers
        .get("X-Real-IP")
        .and_then(|val| val.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    // 1. Fetch data (I/O)
    let user_model = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    // 2. Async logic (Password hashing)
    let is_valid = hasher::verify_password(payload.password, user_model.password_hash.clone()).await?;

    // 3. Decision Logic (Pure)
    let effect = login::decide_login(&user_model, is_valid, access_token_ttl, session_ttl, &encoding_key)?;

    // 4. Execution (I/O)
    let active_session = sessions::ActiveModel {
        id: Set(effect.session_id),
        user_id: Set(effect.user_id),
        refresh_token_hash: Set(effect.refresh_token_hash.clone()),
        ip_address: Set(ip_address),
        device_fingerprint: Set(effect.user_id.to_string()),
        created_at: Set(Utc::now()),
        expires_at: Set(effect.expires_at),
        ..Default::default()
    };

    active_session.insert(db.as_ref()).await?;

    if let Some(client) = valkey_client {
        let mut conn = client.get_connection();
        let whitelist_key = format!("whitelist:session:{}", effect.session_id);
        let _: () = redis::AsyncCommands::set_ex(&mut conn, &whitelist_key, &effect.refresh_token_hash, session_ttl.0 as u64)
            .await?;
    }

    Ok(Json(ApiResponse::success(200, "Login successful", effect.response_data)))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = UserRegistrationRequest,
    responses(
        (status = 201, description = "Registration successful", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 409, description = "Conflict Validation Error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn register_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. Fetch data (I/O)
    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;
    
    let customer_role = roles::Entity::find()
        .filter(roles::Column::Name.eq("Customer"))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing")))?;

    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?;

    // 2. Async logic (Password hashing)
    let hashed_password = hasher::hash_password(payload.password.clone()).await?;

    // 3. Decision Logic (Pure)
    let effect = register::decide_register(payload, existing.as_ref(), pending_status.id, customer_role.id, hashed_password)?;

    // 4. Execution (I/O)
    let now = Utc::now();
    let user_active = users::ActiveModel {
        id: Set(effect.user_id),
        full_name: Set(effect.full_name.clone()),
        email: Set(effect.email.clone()),
        password_hash: Set(effect.hashed_password),
        phone_number: Set(effect.phone_number),
        role_id: Set(effect.role_id),
        account_status: Set(effect.account_status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    if effect.is_new {
        user_active.insert(db.as_ref()).await?;
    } else {
        user_active.update(db.as_ref()).await?;
    }

    if let Some(client) = valkey_client {
        let mut conn = client.get_connection();
        let valkey_key = format!("register_verification:{}", effect.email);
        let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, &templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
    }

    Ok(Json(ApiResponse::message_only(201, "Registration successful")))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-otp",
    request_body = VerifyOtpRequest,
    responses(
        (status = 200, description = "Account verified successfully", body = MessageOnlyResponse),
        (status = 400, description = "Invalid OTP", body = ErrorResponse),
        (status = 403, description = "Too many failed attempts", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn verify_otp_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. I/O: Interact with Valkey
    let client = valkey_client.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let mut conn = client.get_connection();
    let script_hashes = client.get_script_hashes();
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("register_verification:{}", payload.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash)
        .arg(1)
        .arg(&valkey_key)
        .arg(&payload.otp_code)
        .query_async(&mut conn)
        .await?;

    // 2. I/O: Fetch user and status info
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?;
        
    let active_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Active"))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing")))?;

    // 3. Decision Logic (Pure)
    let effect = verify_otp::decide_verify_otp(result, user.as_ref(), active_status.id)?;

    // 4. Execution (I/O)
    let user_active = users::ActiveModel {
        id: Set(effect.user_id),
        account_status: Set(effect.active_status_id),
        ..Default::default()
    };
    user_active.update(db.as_ref()).await?;

    if let Some(rmq) = rabbitmq {
        email_service::send_welcome_email(&rmq, &templates, &effect.email, &effect.full_name).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "Verified successfully")))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/resend-otp",
    request_body = ResendOtpRequest,
    responses(
        (status = 200, description = "OTP resent successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
pub async fn resend_otp_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<ResendOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    
    // 1. Fetch data (I/O)
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?;
    
    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;

    // 2. Decision Logic (Pure)
    let effect = resend_otp::decide_resend_otp(user.as_ref(), pending_status.id, payload)?;

    // 3. Execution (I/O)
    if let Some(client) = valkey_client {
        let mut conn = client.get_connection();
        let valkey_key = format!("register_verification:{}", effect.email);
        let valkey_data = serde_json::json!({ "code": effect.otp_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, &templates, &effect.email, &effect.full_name, &effect.otp_code).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "OTP resent")))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/register", post(register_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/resend-otp", post(resend_otp_handler))
        .route("/refresh-token", post(refresh_token_handler))
        .route("/forgot-password", post(forgot_password_handler))
        .route("/verify-forgot-password-otp", post(verify_forgot_password_otp_handler))
        .route("/reset-password", post(reset_password_handler))
}
