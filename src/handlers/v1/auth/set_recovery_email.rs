use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait};
use validator::Validate;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::auth::set_recovery_email_request::SetRecoveryEmailRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::services::v1::auth::set_recovery_email::decide_set_recovery_email;
use crate::services::v1::core::email_service;
use crate::utils::{hasher, otp};
use redis::AsyncCommands;

#[utoipa::path(
    post,
    path = "/api/v1/auth/recovery-email",
    tag = "auth",
    request_body = SetRecoveryEmailRequest,
    responses(
        (status = 200, description = "OTP sent to recovery email", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn set_recovery_email_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_connection): State<Option<Arc<lapin::Connection>>>,
    AuthUser { user, .. }: AuthUser,
    Json(payload): Json<SetRecoveryEmailRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let password_valid = hasher::verify_password(payload.password, user.password_hash.clone()).await?;

    let effect = decide_set_recovery_email(&user, password_valid, payload.recovery_email.clone())?;

    let recovery_otp = otp::generate_6digit_otp();

    // Cache the OTP in Valkey keyed by user_id
    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let cache_key = format!("recovery_email_verify:{}", user.id);
            let cache_data = serde_json::json!({
                "code": recovery_otp,
                "email": effect.recovery_email,
                "attempts": 5,
            }).to_string();
            let _: () = conn.set_ex(&cache_key, cache_data, 600).await.unwrap_or_else(|e| {
                tracing::warn!("Failed to cache recovery email OTP: {:?}", e);
            });
        }
    }

    // Send OTP to the recovery email
    if let Some(rmq) = rabbitmq_connection {
        email_service::send_verification_email(
            &rmq,
            &std::collections::HashMap::new(),
            &effect.recovery_email,
            &user.full_name,
            &recovery_otp,
        ).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "OTP sent to recovery email")))
}
