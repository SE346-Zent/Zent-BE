use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::deny_part_request::DenyPartRequest;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::services::v1::inventory::deny_part;
use crate::entities::new_part_forms;
use sea_orm::{EntityTrait, ActiveModelTrait, TransactionTrait, Set};
use chrono::Utc;
use validator::Validate;

/// Deny a pending part registration form with a reason.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/parts/{id}/deny",
    tag = "inventory",
    request_body = DenyPartRequest,
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the part registration form")
    ),
    responses(
        (status = 200, description = "Part registration denied successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Part form not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn deny_part(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(payload): Json<DenyPartRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let form = new_part_forms::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Part form not found".to_string()))?;

    let current_form_status = form.status.clone();
    let form_id = form.id;

    let effect = deny_part::decide_deny_part(
        form_id,
        auth.user.id,
        &current_form_status,
        &payload.reason,
        Utc::now(),
    )?;

    state.db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        let form_update = new_part_forms::ActiveModel {
            id: Set(form_id),
            status: Set("rejected".to_string()),
            new_part_request_status_id: Set(3), // rejected
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        form_update.update(txn).await?;
        effect.denial_audit_model.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Part registration form denied successfully")))
}
