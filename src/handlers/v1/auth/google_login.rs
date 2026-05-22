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
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::{users, sessions};
use chrono::Utc;
use crate::utils::hasher;
use crate::services::v1::auth::google_login;
use crate::model::requests::auth::google_login_request::GoogleLoginRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::auth::login_response::LoginResponseData;

#[utoipa::path(
    post,
    path = "/api/v1/auth/google-login",
    request_body = GoogleLoginRequest,
    responses(
        (status = 200, description = "Google login successful", body = ApiResponse<LoginResponseData>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
/// Handle Google/Firebase authentication requests by verifying the ID token.
///
/// If the token is valid and the user does not exist in the system,
/// they will be automatically registered as a Customer with Active status.
/// A session is then created and whitelisted in the Valkey cache.
pub async fn google_login_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(access_token_ttl): State<AccessTokenDefaultTTLSeconds>,
    State(session_ttl): State<SessionDefaultTTLSeconds>,
    State(encoding_key): State<EncodingKey>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<GoogleLoginRequest>,
) -> Result<Json<ApiResponse<LoginResponseData>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let client_ip_address = headers.get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    // 1. Verify token and extract claims
    let project_id = google_login::get_firebase_project_id().unwrap_or_default();
    let claims = google_login::verify_google_or_firebase_token(&payload.id_token, &project_id).await?;

    let email = claims.email.ok_or_else(|| AppError::BadRequest("ID token does not contain email".to_string()))?;

    // 2. Load required lookup constants
    let active_status_id = *lookup_tables.account_statuses_by_name.get("Active")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing in cache")))?;

    let customer_role_id = *lookup_tables.roles_by_name.get("Customer")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing in cache")))?;

    // 3. Find existing user
    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(db.as_ref())
        .await?;

    // 4. If new user, generate a secure random password hash placeholder
    let hashed_placeholder = if existing_user.is_none() {
        let raw_uuid = uuid::Uuid::new_v4().to_string();
        hasher::hash_password(raw_uuid).await?
    } else {
        "".to_string()
    };

    // 5. Execute decision service
    let login_effect = google_login::decide_google_login(
        existing_user.as_ref(),
        email,
        claims.name,
        claims.picture,
        active_status_id,
        customer_role_id,
        hashed_placeholder,
        access_token_ttl,
        session_ttl,
        &encoding_key,
    )?;

    // 6. Persist DB updates (insert user if new, update status if pending)
    if let Some(user_model) = login_effect.user_active_model {
        if existing_user.is_none() {
            user_model.insert(db.as_ref()).await?;
        } else {
            user_model.update(db.as_ref()).await?;
        }
    }

    // 7. Save active session
    let active_session = sessions::ActiveModel {
        id: Set(login_effect.session_id),
        user_id: Set(login_effect.user_id),
        refresh_token_hash: Set(login_effect.refresh_token_hash.clone()),
        ip_address: Set(client_ip_address),
        device_fingerprint: Set(login_effect.user_id.to_string()),
        created_at: Set(Utc::now()),
        expires_at: Set(login_effect.session_expires_at),
        ..Default::default()
    };
    active_session.insert(db.as_ref()).await?;

    // 8. Whitelist in Valkey
    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let whitelist_key = format!("whitelist:session:{}", login_effect.session_id);
            let _: () = redis::AsyncCommands::set_ex(
                &mut conn,
                &whitelist_key,
                &login_effect.refresh_token_hash,
                session_ttl.0 as u64,
            )
            .await
            .unwrap_or_default();
        } else {
            tracing::warn!("Valkey unavailable — session whitelist not set");
        }
    }

    Ok(Json(ApiResponse::success(200, "Google login successful", login_effect.response_data)))
}
