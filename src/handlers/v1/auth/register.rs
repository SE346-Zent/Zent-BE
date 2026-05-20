use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, *};
use validator::Validate;
use chrono::Utc;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::infrastructure::cache::ValkeyClient;
use crate::entities::users;
use crate::utils::hasher;
use crate::services::v1::auth::register;
use crate::services::v1::core::email_service;
use crate::model::requests::auth::user_registration_request::UserRegistrationRequest;
use redis::AsyncCommands;

use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = UserRegistrationRequest,
    responses(
        (status = 201, description = "Registration successful", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 409, description = "Conflict Validation Error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    )
)]
/// Handle new user registration requests by validating data, hashing passwords, and initiating verification.
///
/// This handler coordinates the registration flow: checking for existing users,
/// persisting the (potentially new) user record in MySQL, caching the verification
/// OTP in Valkey, and queuing a verification email via RabbitMQ.
///
/// # Arguments
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables.
/// * `valkey_client` - Optional shared Valkey client for OTP caching.
/// * `rabbitmq_connection` - Optional shared RabbitMQ connection for email delivery.
/// * `email_templates` - Shared pre-loaded HTML email templates.
/// * `registration_payload` - The user's registration details (name, email, password, phone).
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn register_handler(
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_connection): State<Option<Arc<lapin::Connection>>>,
    State(email_templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(registration_payload): Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    registration_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;

    let pending_status_id = *lookup_tables.account_statuses_by_name.get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing in cache")))?;

    let customer_role_id = *lookup_tables.roles_by_name.get("Customer")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing in cache")))?;

    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&registration_payload.email))
        .one(db_connection.as_ref()).await?;

    let hashed_password = hasher::hash_password(registration_payload.password.clone()).await?;
    let registration_effect = register::decide_register(registration_payload, existing_user.as_ref(), pending_status_id, customer_role_id, hashed_password)?;

    let current_time = Utc::now();
    let mut user_active_model = users::ActiveModel {
        id: Set(registration_effect.user_id),
        full_name: Set(registration_effect.full_name.clone()),
        email: Set(registration_effect.email_address.clone()),
        password_hash: Set(registration_effect.hashed_password),
        phone_number: Set(registration_effect.phone_number),
        role_id: Set(registration_effect.role_id),
        account_status: Set(registration_effect.account_status),
        updated_at: Set(current_time),
        ..Default::default()
    };

    if registration_effect.is_new_record {
        user_active_model.created_at = Set(current_time);
        user_active_model.insert(db_connection.as_ref()).await?;
    } else {
        user_active_model.update(db_connection.as_ref()).await?;
    }

    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let valkey_key = format!("register_verification:{}", registration_effect.email_address);
            let valkey_data = serde_json::json!({ "code": registration_effect.verification_otp, "attempts": 5 }).to_string();
            let _ = conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await;
        } else {
            tracing::warn!("Valkey unavailable — OTP not cached, user can retry");
        }
    }

    if let Some(rabbitmq) = rabbitmq_connection {
        email_service::send_verification_email(&rabbitmq, &email_templates, &registration_effect.email_address, &registration_effect.full_name, &registration_effect.verification_otp).await?;
    }

    Ok(Json(ApiResponse::message_only(201, "Registration successful")))
}
