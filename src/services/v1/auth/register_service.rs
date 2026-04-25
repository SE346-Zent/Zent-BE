use crate::{
    entities::users,
    errors::AppError,
    infrastructure::mq::publish_email_message,
    model::responses::base::ApiResponse,
    repository::{account_status_repository, role_repository, user_repository},
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use crate::model::requests::auth::user_registration_request::UserRegistrationRequest;
use sea_orm::*;
use uuid::Uuid;

pub async fn perform_register(
    db: DatabaseConnection,
    rabbitmq: std::sync::Arc<lapin::Connection>,
    req: UserRegistrationRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Check if user already exists with email via repository
    let existing_user = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with email already exists".to_string(),
        ));
    }

    // 2. Load Customer role via repository
    let customer_role = role_repository::find_by_name(&db, "Customer")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading role: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role not found")))?;

    // 3. Load Pending account status via repository
    let pending_status = account_status_repository::find_by_name(&db, "Pending")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

    // 4. Hash the password
    let password_bytes = req.password.into_bytes();
    let hashed_password = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(&password_bytes, &salt)
            .map(|hash| hash.to_string())
    })
    .await
    .map_err(|_| AppError::Internal(anyhow::anyhow!("Failed to spawn blocking task")))?
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Password hashing failed: {}", e)))?;

    let now = Utc::now();

    // 5. Persist user via repository
    let new_user = users::ActiveModel {
        id: Set(Uuid::new_v4()),
        full_name: Set(req.full_name.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(hashed_password),
        phone_number: Set(req.phone_number),
        account_status: Set(pending_status.id),
        role_id: Set(customer_role.id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    user_repository::create(&db, new_user)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create user: {:?}", e)))?;

    // 6. Deliver async email task to RabbitMQ
    let email_payload = serde_json::json!({
        "to": req.email,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Please verify your email.", req.full_name)
    });
    if let Err(e) = publish_email_message(&rabbitmq, email_payload.to_string().as_bytes()).await {
        tracing::error!("Failed to enqueue registration email task into RabbitMQ: {}", e);
    }

    // 7. Finalise
    Ok(ApiResponse::message_only(201, "Registration successful"))
}
