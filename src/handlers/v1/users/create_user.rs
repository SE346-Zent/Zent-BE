use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    model::requests::users::UserCreateRequest,
    model::responses::base::ApiResponse,
    model::responses::users::UserResponseData,
    services::v1::users::create_user,
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

    // Pure logic: RBAC + prepare ActiveModel
    let effect = create_user::decide_can_create_user(current_user, payload)?;

    // Hash password
    let plain_password = effect.plain_password.clone();
    let hashed = if let Some(ref plain) = plain_password {
        hasher::hash_password(plain.clone()).await?
    } else {
        // Fallback: generate one
        let generated = crate::utils::otp::generate_6digit_otp();
        hasher::hash_password(generated).await?
    };

    let mut user_active_model = effect.user_active_model;
    user_active_model.password_hash = Set(hashed);

    let user_model = user_active_model.insert(db.as_ref()).await?;

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
