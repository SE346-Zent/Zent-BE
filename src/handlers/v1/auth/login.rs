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
use chrono::Utc;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::{users, sessions};
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

    let ip_address = headers.get("X-Real-IP").and_then(|v| v.to_str().ok())
        .map(|s| s.to_string()).unwrap_or_else(|| addr.ip().to_string());

    let user_model = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::Unauthorized("Invalid credentials".to_string()))?;

    let is_valid = hasher::verify_password(payload.password, user_model.password_hash.clone()).await?;
    let effect = login::decide_login(&user_model, is_valid, access_token_ttl, session_ttl, &encoding_key)?;

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
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", effect.session_id);
            let _: () = redis::AsyncCommands::set_ex(&mut conn, &whitelist_key, &effect.refresh_token_hash, session_ttl.0 as u64).await.unwrap_or_default();
        } else {
            tracing::warn!("Valkey unavailable — session whitelist not set");
        }
    }

    Ok(Json(ApiResponse::success(200, "Login successful", effect.response_data)))
}
