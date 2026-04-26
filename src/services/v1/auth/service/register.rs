use sea_orm::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::{
    core::errors::AppError,
    entities::users,
    model::{
        requests::auth::user_registration_request::UserRegistrationRequest,
        responses::base::ApiResponse,
    },
    repository::{user_repository, account_status_repository, role_repository},
    services::v1::core::email_service,
    infrastructure::mq::RabbitMQManager,
};
use crate::utils::{hasher, otp};
use uuid::Uuid;
use redis::AsyncCommands;

pub async fn handle_register(
    db: DatabaseConnection,
    valkey: Option<redis::aio::MultiplexedConnection>,
    rabbitmq: Option<Arc<RabbitMQManager>>,
    templates: &HashMap<String, String>,
    req: UserRegistrationRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Check existing
    let existing = user_repository::find_by_email(&db, &req.email).await?;

    if let Some(user) = existing {
        if user.account_status != 1 { // Assuming 1 is Pending
            return Err(AppError::Conflict("Email already registered and active".to_string()));
        }
    }

    // 2. Hash password
    let hashed_password = hasher::hash_password(req.password).await?;

    // 3. Load roles/status
    let customer_role = role_repository::find_by_name(&db, "Customer").await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role missing")))?;
    let pending_status = account_status_repository::find_by_name(&db, "Pending").await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status missing")))?;

    // 4. Save user
    let user_id = Uuid::new_v4();
    let user_active = users::ActiveModel {
        id: Set(user_id),
        full_name: Set(req.full_name.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(hashed_password),
        phone_number: Set(req.phone_number.clone()),
        role_id: Set(customer_role.id),
        account_status: Set(pending_status.id),
        ..Default::default()
    };

    user_repository::create(&db, user_active).await?;

    // 5. OTP
    let verification_code = otp::generate_6digit_otp();

    if let Some(mut conn) = valkey {
        let valkey_key = format!("register_verification:{}", req.email);
        let valkey_data = serde_json::json!({ "code": verification_code, "attempts": 5 }).to_string();
        conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await?;
    }

    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(&rmq, templates, &req.email, &req.full_name, &verification_code).await?;
    }

    Ok(ApiResponse::message_only(201, "Registration successful"))
}
