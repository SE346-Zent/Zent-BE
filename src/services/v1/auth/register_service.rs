use crate::{
    entities::{account_status, roles, users},
    infrastructure::mq::publish_email_message,
    model::{
        requests::auth::user_registration_request::UserRegistrationRequest,
        responses::{auth::register_response::RegisterResponse, error::AppError},
    }
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

pub async fn perform_register(
    db: DatabaseConnection,
    rabbitmq: std::sync::Arc<lapin::Connection>,
    req: UserRegistrationRequest,
) -> Result<RegisterResponse, AppError> {
    // 3. Check if user already exist with email
    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&req.email))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if existing_user.is_some() {
        return Err(AppError::Conflict(
            "User with email already exists".to_string(),
        ));
    }

    // 4. Construct User entity object
    let customer_role = roles::Entity::find()
        .filter(roles::Column::Name.eq("Customer"))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading role: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role not found")))?;

    let pending_status = account_status::Entity::find()
        .filter(account_status::Column::Name.eq("Pending"))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

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

    let new_user = users::ActiveModel {
        id: Set(Uuid::new_v4()),
        full_name: Set(req.full_name.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(hashed_password),
        phone_number: Set(req.phone_number),
        account_status: Set(pending_status.id), // Using id from pending_status
        role_id: Set(customer_role.id),         // Using id from customer_role
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    // 5. Persist record
    new_user
        .insert(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create user: {:?}", e)))?;

    // 6. Deliver async email task to DLQ RabbitMQ environment securely bypassing direct blocking HTTP flows.
    let email_payload = serde_json::json!({
        "to": req.email,
        "subject": "Welcome to Zent!",
        "body": format!("Welcome to Zent, {}! Please verify your email.", req.full_name)
    });
    if let Err(e) = publish_email_message(&rabbitmq, email_payload.to_string().as_bytes()).await {
        tracing::error!("Failed to enqueue registration email task into RabbitMQ: {}", e);
    }

    // 7. Finalise
    Ok(RegisterResponse::success("Registration successful"))
}
