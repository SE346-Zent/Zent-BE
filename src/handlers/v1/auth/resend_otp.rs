use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::users;
use crate::services::v1::auth::resend_otp;
use crate::services::v1::core::email_service;
use crate::model::requests::auth::resend_otp_request::ResendOtpRequest;
use redis::AsyncCommands;

use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

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
/// Handle requests to resend a verification OTP for a pending account.
///
/// This handler verifies that the user exists and is still in a pending state,
/// generates a new OTP, caches it in Valkey (overwriting any previous one),
/// and queues a new verification email via RabbitMQ.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables.
/// * `valkey_client` - Optional shared Valkey client for OTP caching.
/// * `rabbitmq_connection` - Optional shared RabbitMQ connection for email delivery.
/// * `email_templates` - Shared pre-loaded HTML email templates.
/// * `resend_payload` - The request containing the user's email address.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn resend_otp_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_connection): State<Option<Arc<lapin::Connection>>>,
    State(email_templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(resend_payload): Json<ResendOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    resend_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let user_record = users::Entity::find()
        .filter(users::Column::Email.eq(&resend_payload.email))
        .one(db_connection.as_ref()).await?;

    let pending_status_id = *lookup_tables.account_statuses_by_name.get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing in cache")))?;

    let resend_effect = resend_otp::decide_resend_otp(user_record.as_ref(), pending_status_id, resend_payload)?;

    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let valkey_key = format!("register_verification:{}", resend_effect.email_address);
            let valkey_data = serde_json::json!({ "code": resend_effect.new_otp_code, "attempts": 5 }).to_string();
            let _ = conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await;
        } else {
            tracing::warn!("Valkey unavailable — resend OTP not cached");
        }
    }

    if let Some(rabbitmq) = rabbitmq_connection {
        email_service::send_verification_email(&rabbitmq, &email_templates, &resend_effect.email_address, &resend_effect.full_name, &resend_effect.new_otp_code).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "OTP resent")))
}
