use axum::{extract::{State, Path}, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait};
use uuid::Uuid;
use redis::AsyncCommands;
use crate::{
    core::errors::AppError,
    entities::users,
    extractor::auth_user::AuthUser,
    infrastructure::cache::ValkeyClient,
    model::requests::users::UserStatusUpdateRequest,
    model::responses::base::ApiResponse,
    services::v1::users::update_status,
};

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}/status",
    tag = "users",
    request_body = UserStatusUpdateRequest,
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the user")
    ),
    responses(
        (status = 200, description = "Update status successful"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_user_status_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    AuthUser { user: current_user, .. }: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UserStatusUpdateRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let target_user = users::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("User account not found".to_string()))?;

    let effect = update_status::decide_can_update_status(current_user, target_user, payload)?;

    effect.user_active_model.update(db.as_ref()).await?;

    // Invalidate cached user profile so login sees fresh status
    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let profile_cache_key = format!("user_profile:{}", id);
            let _: () = conn.del(&profile_cache_key).await.unwrap_or_default();
        }
    }

    Ok(Json(ApiResponse::success(200, "Update status successful", ())))
}
