use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::accept_part;
use crate::entities::new_part_forms;
use sea_orm::{EntityTrait, ActiveModelTrait, TransactionTrait, Set};
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

    let current_form_status = form.status.clone();
    let form_id = form.id;
    let form_description = form.description.clone();
    let form_part_number = form.part_number.clone();
    let form_serial_number = form.serial_number.clone();
    let form_created_at = form.created_at;

    let effect = accept_part::decide_accept_part(
        form_id,
        auth.user.id,
        &current_form_status,
        Utc::now(),
    )?;

    // Ensure part catalog exists in Zeus. If not found, create it. If found, use existing.
    let found_catalog = state
        .zeus_client
        .find_part_catalog_by_part_number(&form.part_number)
        .await?;

    // Validate part_number is not empty before creating catalog
    if form.part_number.trim().is_empty() {
        return Err(AppError::BadRequest("Part number cannot be empty".to_string()));
    }

    let catalog = if let Some(existing) = found_catalog {
        // Use existing catalog - no need to update
        existing
    } else {
        // Create a minimal catalog entry when missing. Use model_code-derived mfg number fallback.
        let mfg_number = form.model_code.clone().unwrap_or_else(|| format!("MFG-{}", form.part_number));
        state
            .zeus_client
            .create_part_catalog(&form_part_number, form.part_types_id, &mfg_number, form_description.as_deref(), 1)
            .await?
    };

    // Condition defaults to 1 (New), mfg_date to form creation date
    let condition_id = 1;
    let mfg_date = form_created_at;

    state.zeus_client.create_part(
        catalog.id,
        condition_id,
        &form_serial_number,
        mfg_date,
    ).await?;

    state.db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        let form_update = new_part_forms::ActiveModel {
            id: Set(form_id),
            status: Set("approved".to_string()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };
        form_update.update(txn).await?;
        effect.approval_audit_model.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Part registration form accepted and synchronized successfully")))
}
