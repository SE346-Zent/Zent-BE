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
pub async fn resend_otp_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<ResendOtpRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?;

    let pending_status_id = *luts.account_statuses_by_name.get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing in cache")))?;

    let effect = resend_otp::decide_resend_otp(user.as_ref(), pending_status_id, payload)?;

    use redis::AsyncCommands;
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
