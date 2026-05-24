use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::inventory::part_detail_response::PartDetailResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::get_part::{self, PartWithRelations};
use super::list_parts::assemble_part_entries;

/// Retrieve detailed information for a single part.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/parts/{id}",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the part")
    ),
    responses(
        (status = 200, description = "Detailed part information retrieved successfully", body = ApiResponse<PartDetailResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Part not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_part(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<PartDetailResponse>>, AppError> {
    let zeus_part = state.zeus_client.get_part(id).await?;
    let mut assembled = assemble_part_entries(&state.db, vec![zeus_part]).await?;
    let part_entry = assembled.pop()
        .ok_or_else(|| AppError::NotFound("Part not found".to_string()))?;

    let part_relation_data = PartWithRelations {
        part_record: part_entry.part_record,
        catalog_definition: part_entry.catalog_definition,
        physical_condition: part_entry.physical_condition,
        installed_product: part_entry.installed_product,
        approval_status: part_entry.approval_status,
        denial_reason: part_entry.denial_reason,
        customer_id: part_entry.customer_id,
        technician_id: part_entry.technician_id,
    };

    let detail = get_part::get_part_detail(&part_relation_data, &auth.role.name, auth.user.id)?;

    Ok(Json(ApiResponse::success(
        200,
        "Part retrieved successfully",
        detail,
    )))
}
