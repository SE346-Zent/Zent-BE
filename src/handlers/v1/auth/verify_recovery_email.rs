use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, EntityTrait, Set};
use validator::Validate;
use crate::core::errors::{AppError, ErrorResponse};
use crate::entities::users;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::auth::verify_recovery_email_request::VerifyRecoveryEmailRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::services::v1::auth::verify_recovery_email::decide_verify_recovery_email;
use redis::AsyncCommands;

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-recovery-email",
    tag = "auth",
    request_body = VerifyRecoveryEmailRequest,
    responses(
        (status = 200, description = "Recovery email verified successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Too many attempts", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn verify_recovery_email_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<VerifyRecoveryEmailRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let client = valkey_client.ok_or_else(|| {
        AppError::ServiceUnavailable("Verification service is temporarily unavailable".to_string())
    })?;
    let mut conn = client.get_connection().await?;
    let script_hashes = client.get_script_hashes();
    let verify_otp_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let cache_key = format!("recovery_email_verify:{}", user.id);
    let lua_result: i32 = redis::cmd("EVALSHA")
        .arg(verify_otp_hash)
        .arg(1)
        .arg(&cache_key)
        .arg(&payload.otp_code)
        .query_async(&mut conn)
        .await?;

    decide_verify_recovery_email(lua_result)?;

    // OTP verified — read the cached recovery email
    let cached: Option<String> = conn.get(&cache_key).await.ok().flatten();
    let recovery_email = if let Some(json_str) = cached {
        serde_json::from_str::<serde_json::Value>(&json_str)
            .ok()
            .and_then(|v| v["email"].as_str().map(|s| s.to_string()))
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to parse cached recovery email")))?
    } else {
        return Err(AppError::BadRequest("OTP session expired. Please try again".to_string()));
    };

    // Update user's recovery_email in DB
    let mut active: users::ActiveModel = user.into();
    active.recovery_email = Set(Some(recovery_email));
    active.update(db.as_ref()).await?;

    Ok(Json(ApiResponse::message_only(200, "Recovery email verified successfully")))
}
