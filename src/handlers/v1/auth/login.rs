use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    Json,
};
use std::net::SocketAddr;
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use jsonwebtoken::EncodingKey;
use validator::Validate;
use uuid::Uuid;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::metrics;
use crate::entities::{login_audit_logs, sessions, users};
use chrono::Utc;
use crate::utils::hasher;
use crate::services::v1::auth::login;
use crate::model::requests::auth::user_login_request::UserLoginRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::auth::login_response::LoginResponseData;

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
/// Handle user login requests by verifying credentials and establishing a session.
///
/// This handler extracts connection info and credentials, verifies the password,
/// calculates the login outcome, and persists the new session in both the
/// relational database (MySQL) and the session whitelist cache (Valkey).
///
/// # Arguments
/// * `db` - Shared database connection pool.
/// * `valkey_client` - Optional shared Valkey client for session whitelisting.
/// * `access_token_ttl` - Default TTL for access tokens from global state.
/// * `session_ttl` - Default TTL for sessions from global state.
/// * `encoding_key` - Key for signing JWTs.
/// * `headers` - HTTP headers (used to extract IP address).
/// * `addr` - Socket address of the connecting client.
/// * `payload` - The user's login credentials (email and password).
///
/// # Returns
/// A result containing the successful `ApiResponse` with login data, or an `AppError`.
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

    let client_ip_address = headers.get("X-Real-IP").and_then(|v| v.to_str().ok())
        .map(|s| s.to_string()).unwrap_or_else(|| addr.ip().to_string());

    let user_record = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid email or password".to_string()))?;

    let is_password_valid = hasher::verify_password(payload.password, user_record.password_hash.clone()).await?;
    let login_effect = login::decide_login(&user_record, is_password_valid, access_token_ttl, session_ttl, &encoding_key)?;

    if let Some(fcm_token) = payload.fcm_token {
        let mut user_active: users::ActiveModel = user_record.into();
        user_active.fcm_token = Set(Some(fcm_token));
        user_active.updated_at = Set(Utc::now());
        user_active.update(db.as_ref()).await?;
    }

    let active_session = sessions::ActiveModel {
        id: Set(login_effect.session_id),
        user_id: Set(login_effect.user_id),
        refresh_token_hash: Set(login_effect.refresh_token_hash.clone()),
        ip_address: Set(client_ip_address.clone()),
        device_fingerprint: Set(payload.device_name.clone().unwrap_or_else(|| login_effect.user_id.to_string())),
        created_at: Set(Utc::now()),
        expires_at: Set(login_effect.session_expires_at),
        ..Default::default()
    };

    let login_audit = login_audit_logs::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(login_effect.user_id),
        session_id: Set(login_effect.session_id),
        device_name: Set(payload.device_name.clone().unwrap_or_else(|| "Unknown device".to_string())),
        location: Set(payload.location.clone()),
        ip_address: Set(client_ip_address),
        created_at: Set(Utc::now()),
    };

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        active_session.insert(txn).await?;
        login_audit.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", login_effect.session_id);
            let _: () = redis::AsyncCommands::set_ex(&mut conn, &whitelist_key, &login_effect.refresh_token_hash, session_ttl.0 as u64).await.unwrap_or_default();
        } else {
            tracing::warn!("Valkey unavailable — session whitelist not set");
        }
    }

    // Track successful login
    metrics::init().auth_login_total.add(1, &[
        opentelemetry::KeyValue::new("method", "password"),
        opentelemetry::KeyValue::new("status", "success"),
    ]);

    Ok(Json(ApiResponse::success(200, "Login successful", login_effect.response_data)))
}
