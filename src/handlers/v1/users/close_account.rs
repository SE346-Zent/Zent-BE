use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait};
use redis::AsyncCommands;
use crate::{
    core::errors::AppError,
    core::lookup_tables::LookupTables,
    extractor::auth_user::AuthUser,
    infrastructure::cache::ValkeyClient,
    model::responses::base::ApiResponse,
    services::v1::users::close_account,
};

#[utoipa::path(
    post,
    path = "/api/v1/users/me/close",
    tag = "users",
    responses(
        (status = 200, description = "Account closed successful"),
        (status = 403, description = "Forbidden"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("jwt" = []))
)]
pub async fn close_account_handler(
    State(db): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    AuthUser { user, .. }: AuthUser,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let terminated_status_id = lookup_tables
        .account_statuses_by_name
        .get("Terminated")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing Terminated account status in lookup tables")))?;

    let customer_role_id = lookup_tables
        .roles_by_name
        .get("Customer")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing Customer role in lookup tables")))?;

    let user_id = user.id;
    let effect = close_account::decide_close_account(user, terminated_status_id, customer_role_id)?;
    effect.user_active_model.update(db.as_ref()).await?;

    // Invalidate cached user profile
    if let Some(client) = valkey_client {
        if let Ok(mut conn) = client.get_connection().await {
            let profile_cache_key = format!("user_profile:{}", user_id);
            let _: () = conn.del(&profile_cache_key).await.unwrap_or_default();
        }
    }

    Ok(Json(ApiResponse::success(200, "Account closed successful", ())))
}
