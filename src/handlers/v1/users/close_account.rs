use axum::{extract::State, Json};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait};
use crate::{
    core::errors::AppError,
    core::lookup_tables::LookupTables,
    extractor::auth_user::AuthUser,
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
    AuthUser { user, .. }: AuthUser,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let terminated_status_id = lookup_tables
        .account_statuses_by_name
        .get("Terminated")
        .copied()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing Terminated account status in lookup tables")))?;

    let effect = close_account::decide_close_account(user, terminated_status_id)?;
    effect.user_active_model.update(db.as_ref()).await?;
    Ok(Json(ApiResponse::success(200, "Account closed successful", ())))
}
