use axum::{extract::State, Json};
use std::sync::Arc;
use std::collections::HashMap;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::requests::users::UserCreateRequest,
    model::responses::base::ApiResponse,
    model::responses::users::UserResponseData,
    services::v1::users::create_user,
    services::v1::core::email_service,
    utils::hasher,
};

#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "users",
    request_body = UserCreateRequest,
    responses(
        (status = 201, description = "User created successful", body = ApiResponse<UserResponseData>),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn create_user_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(rabbitmq): State<Option<Arc<lapin::Connection>>>,
    State(_templates): State<Arc<HashMap<String, String>>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Json(payload): Json<UserCreateRequest>,
) -> Result<Json<ApiResponse<UserResponseData>>, AppError> {
    // Check for duplicate email
    let existing = users::Entity::find()
        .filter(users::Column::Email.eq(&payload.email))
        .one(db.as_ref())
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("A user with this email already exists".to_string()));
    }

    // Pure logic: RBAC + prepare ActiveModel (password auto-generated from UUID)
    let effect = create_user::decide_can_create_user(current_user, payload)?;

    // Hash the auto-generated password
    let plain_password = effect.plain_password.clone().unwrap_or_default();
    let hashed = hasher::hash_password(plain_password.clone()).await?;

    let mut user_active_model = effect.user_active_model;
    user_active_model.password_hash = Set(hashed);

    let user_model = user_active_model.insert(db.as_ref()).await?;

    // Send welcome email with credentials
    if let Some(ref rabbitmq_conn) = rabbitmq {
        let body = format!(
            "Your account has been created.\n\nUsername: {}\nPassword: {}\n\nPlease log in and change your password.",
            user_model.email, plain_password
        );
        let _ = email_service::send_email(
            rabbitmq_conn,
            &user_model.email,
            "Your Zent Account",
            &body,
        ).await;
    }

    Ok(Json(ApiResponse::success(201, "User created successful", UserResponseData {
        id: user_model.id,
        role_id: user_model.role_id,
        full_name: user_model.full_name,
        email: user_model.email,
        phone: Some(user_model.phone_number),
        province: user_model.province,
        account_status_id: user_model.account_status,
        created_at: user_model.created_at.to_rfc3339(),
        updated_at: user_model.updated_at.to_rfc3339(),
    })))
}
