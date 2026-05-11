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
pub async fn register_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Json(payload): Json<UserRegistrationRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let pending_status_id = *luts.account_statuses_by_name.get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing in cache")))?;

    let customer_role_id = *luts.roles_by_name.get("Customer")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing in cache")))?;

    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref()).await?;

    let hashed_password = hasher::hash_password(payload.password.clone()).await?;
    let effect = register::decide_register(payload, existing.as_ref(), pending_status_id, customer_role_id, hashed_password)?;

    let now = Utc::now();
    let mut user_active = users::ActiveModel {
        id: Set(effect.user_id),
        full_name: Set(effect.full_name.clone()),
        email: Set(effect.email.clone()),
        password_hash: Set(effect.hashed_password),
        phone_number: Set(effect.phone_number),
        role_id: Set(effect.role_id),
        account_status: Set(effect.account_status),
        updated_at: Set(now),
        ..Default::default()
    };

    if effect.is_new {
        user_active.created_at = Set(now);
        user_active.insert(db.as_ref()).await?;
    } else {
        user_active.update(db.as_ref()).await?;
    }

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

    Ok(Json(ApiResponse::message_only(201, "Registration successful")))
}
