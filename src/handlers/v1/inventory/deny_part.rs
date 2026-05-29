use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::deny_part_request::DenyPartRequest;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::deny_part;
use crate::entities::{new_part_forms, part_audit_log};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait};
use uuid::Uuid;
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
        (status = 200, description = "Part registration denied successfully", body = ApiResponse<String>),
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

    let audit_log = part_audit_log::Entity::find()
        .filter(part_audit_log::Column::NewPartFormId.eq(form.id))
        .one(state.db.as_ref())
        .await?;

    let current_form_status = if let Some(log) = audit_log {
        log.action.clone()
    } else {
        "pending".to_string()
    };

    let effect = deny_part::decide_deny_part(
        form.id,
        auth.user.id,
        &current_form_status,
        &payload.reason,
        Utc::now(),
    )?;

    effect.denial_audit_model.insert(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::message_only(200, "Part registration form denied successfully")))
}
