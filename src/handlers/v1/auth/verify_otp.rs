use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::users;
use crate::services::v1::auth::verify_otp;
use crate::services::v1::core::email_service;
use crate::model::requests::auth::verify_otp_request::VerifyOtpRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

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
/// Handle requests to verify the registration OTP and activate a user account.
///
/// This handler executes a Valkey Lua script to verify the OTP. If valid, it
/// updates the user's status to 'Active' in MySQL and queues a welcome email
/// via RabbitMQ.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables.
/// * `valkey_client` - Optional shared Valkey client for OTP verification.
/// * `rabbitmq_connection` - Optional shared RabbitMQ connection for email delivery.
/// * `email_templates` - Shared pre-loaded HTML email templates.
/// * `verify_payload` - The request containing the user's email and the OTP code.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn verify_otp_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_connection): State<Option<Arc<lapin::Connection>>>,
    State(email_templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(verify_payload): Json<VerifyOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    verify_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::ServiceUnavailable("Verification service is temporarily unavailable. Please try again later.".to_string()))?;
    let mut valkey_conn = client.get_connection().await?;
    let script_hashes = client.get_script_hashes();
    let verify_otp_script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_otp_key = format!("register_verification:{}", verify_payload.email);
    let lua_result: i32 = redis::cmd("EVALSHA")
        .arg(verify_otp_script_hash).arg(1).arg(&valkey_otp_key).arg(&verify_payload.otp_code)
        .query_async(&mut valkey_conn).await?;

    if lua_result != 1 {
        verify_otp::decide_verify_otp(lua_result, None, 0)?;
        return Err(AppError::Internal(anyhow::anyhow!("Logic error")));
    }

    let user_record = users::Entity::find()
        .filter(users::Column::Email.eq(&verify_payload.email))
        .one(db_connection.as_ref()).await?;

    let active_status_id = *lookup_tables.account_statuses_by_name.get("Active")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing in cache")))?;

    let verify_effect = verify_otp::decide_verify_otp(lua_result, user_record.as_ref(), active_status_id)?;

    let user_active_model = users::ActiveModel {
        id: Set(verify_effect.verified_user_id),
        account_status: Set(verify_effect.target_active_status_id),
        ..Default::default()
    };
    user_active_model.update(db_connection.as_ref()).await?;

    if let Some(rabbitmq) = rabbitmq_connection {
        email_service::send_welcome_email(&rabbitmq, &email_templates, &verify_effect.user_email, &verify_effect.user_full_name).await?;
    }

    Ok(Json(ApiResponse::message_only(200, "Verified successfully")))
}
