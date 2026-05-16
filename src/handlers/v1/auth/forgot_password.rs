use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::services::v1::auth::forgot_password;
use crate::services::v1::core::email_service;
use redis::AsyncCommands;
use sea_orm::EntityTrait;

use crate::entities::users;
use crate::model::requests::auth::forgot_password_request::ForgotPasswordRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

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
/// Handle requests to initiate the forgotten password recovery flow.
///
/// This handler validates the request, checks for user existence, and if the
/// user is found, generates a recovery OTP, caches it in Valkey, and queues
/// a recovery email via RabbitMQ.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `valkey_client` - Optional shared Valkey client for OTP caching.
/// * `rabbitmq_connection` - Optional shared RabbitMQ connection for email delivery.
/// * `email_templates` - Shared pre-loaded HTML email templates.
/// * `forgot_password_payload` - The recovery request containing the user's email.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn forgot_password_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_connection): State<Option<Arc<lapin::Connection>>>,
    State(email_templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(forgot_password_payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    forgot_password_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let user_record = users::Entity::find()
        .filter(users::Column::Email.eq(&forgot_password_payload.email))
        .one(db_connection.as_ref()).await?;

    let forgot_password_effect = forgot_password::decide_forgot_password(user_record.as_ref(), forgot_password_payload)?;

    if let Some(effect) = forgot_password_effect {
        if let Some(client) = valkey_client {
            if let Ok(mut conn) = client.get_connection().await {
                let valkey_key = format!("forgot_password_verification:{}", effect.email_address);
                let valkey_data = serde_json::json!({ "code": effect.recovery_otp, "attempts": 5 }).to_string();
                let _ = conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await;
            } else {
                tracing::warn!("Valkey unavailable — forgot-password OTP not cached");
            }
        }
        if let Some(rabbitmq) = rabbitmq_connection {
            email_service::send_forgot_password_email(&rabbitmq, &email_templates, &effect.email_address, &effect.full_name, &effect.recovery_otp).await?;
        }
    }

    Ok(Json(ApiResponse::message_only(200, "OTP sent")))
}
