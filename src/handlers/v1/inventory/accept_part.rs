use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::accept_part::{self, AcceptPartEffect};
use crate::entities::{new_part_forms, part_audit_log};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use uuid::Uuid;
use chrono::Utc;

/// Accept a pending part registration form and synchronize it with Zeus SCM.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/parts/{id}/accept",
    tag = "inventory",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the part registration form")
    ),
    responses(
        (status = 200, description = "Part registration accepted successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Part form not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn accept_part(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let form = new_part_forms::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Part form not found".to_string()))?;

    let audit_log = part_audit_log::Entity::find()
        .filter(part_audit_log::Column::NewPartFormId.eq(form.id))
        .one(state.db.as_ref())
        .await?;

    let current_form_status = if let Some(log) = audit_log {
        log.action.clone()
    } else {
        "pending".to_string()
    };

    let effect = accept_part::decide_accept_part(
        form.id,
        auth.user.id,
        &current_form_status,
        Utc::now(),
    )?;

    let catalog = state
        .zeus_client
        .find_part_catalog_by_part_number(&form.part_number)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Part number {} not found in SCM catalog", form.part_number)))?;

    // Condition defaults to 1 (New), mfg_date to form creation date
    let condition_id = 1;
    let mfg_date = form.created_at;

    state.zeus_client.create_part(
        catalog.id,
        condition_id,
        &form.serial_number,
        mfg_date,
    ).await?;

    effect.approval_audit_model.insert(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::message_only(200, "Part registration form accepted and synchronized successfully")))
}
