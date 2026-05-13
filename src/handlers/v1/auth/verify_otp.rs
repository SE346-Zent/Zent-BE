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
pub async fn verify_otp_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let client = valkey_client.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Valkey missing")))?;
    let mut conn = client.get_connection();
    let script_hashes = client.get_script_hashes();
    let script_hash = script_hashes.get("verify_otp")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Script hash missing")))?;

    let valkey_key = format!("register_verification:{}", payload.email);
    let result: i32 = redis::cmd("EVALSHA")
        .arg(script_hash).arg(1).arg(&valkey_key).arg(&payload.otp_code)
        .query_async(&mut conn).await?;

    if result != 1 {
        verify_otp::decide_verify_otp(result, None, 0)?;
        return Err(AppError::Internal(anyhow::anyhow!("Logic error")));
    }

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?;

    let active_status_id = *luts.account_statuses_by_name.get("Active")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Active status missing in cache")))?;

    let effect = verify_otp::decide_verify_otp(result, user.as_ref(), active_status_id)?;

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
