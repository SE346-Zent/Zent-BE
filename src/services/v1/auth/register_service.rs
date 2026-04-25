use crate::{
    entities::users,
    core::errors::AppError,
    model::responses::base::ApiResponse,
    repository::{account_status_repository, role_repository, user_repository},
    services::v1::core::email_service,
};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::Utc;
use crate::model::requests::auth::user_registration_request::UserRegistrationRequest;
use sea_orm::*;
use uuid::Uuid;
use redis::AsyncCommands;
use rand::Rng;

pub async fn perform_register(
    db: DatabaseConnection,
    valkey: Option<redis::Client>,
    rabbitmq: Option<std::sync::Arc<lapin::Connection>>,
    req: UserRegistrationRequest,
) -> Result<ApiResponse<()>, AppError> {
    // 1. Load Customer role via repository
    let customer_role = role_repository::find_by_name(&db, "Customer")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading role: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Customer role not found")))?;

    // 2. Load Pending account status via repository
    let pending_status = account_status_repository::find_by_name(&db, "Pending")
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error loading account status: {}", e)))?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

    // 3. Check if user already exists with email via repository
    let existing_user = user_repository::find_by_email(&db, &req.email)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    if let Some(ref user) = existing_user {
        if user.account_status != pending_status.id {
            return Err(AppError::Conflict(
                "User with email already exists".to_string(),
            ));
        }
    }

    // 4. Hash the password
    let password_bytes = req.password.clone().into_bytes();
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

    // 5. Persist user via repository (Create or Update if Pending)
    if let Some(user) = existing_user {
        let mut user_active: users::ActiveModel = user.into_active_model();
        user_active.full_name = Set(req.full_name.clone());
        user_active.password_hash = Set(hashed_password);
        user_active.phone_number = Set(req.phone_number);
        user_active.account_status = Set(pending_status.id);
        user_active.role_id = Set(customer_role.id);
        user_active.updated_at = Set(now);
        user_active.deleted_at = Set(None);

        user_repository::update(&db, user_active)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to update user: {:?}", e)))?;
    } else {
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
    }

    // 6. Generate 6-digit verification code
    let verification_code_str = {
        let mut rng = rand::rng();
        let verification_code: u32 = rng.random_range(100_000..=999_999);
        verification_code.to_string()
    };

    // 7. Store verification code in Valkey if provided
    if let Some(vk) = valkey {
        let valkey_key = format!("register_verification:{}", req.email);
        let valkey_data = serde_json::json!({
            "code": verification_code_str,
            "attempts": 5
        }).to_string();

        let mut valkey_conn = vk.get_multiplexed_async_connection().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to connect to Valkey: {}", e)))?;
            
        valkey_conn.set_ex::<_, _, ()>(&valkey_key, valkey_data, 600).await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to store verification code in Valkey: {}", e)))?;
    }

    // 8. Deliver async email task to RabbitMQ if provided
    if let Some(rmq) = rabbitmq {
        email_service::send_verification_email(
            &rmq,
            &req.email,
            &req.full_name,
            &verification_code_str,
        ).await?;
    }

    // 9. Finalise
    Ok(ApiResponse::message_only(201, "Registration successful"))
}
