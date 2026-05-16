use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::responses::inventory::part_detail_response::PartDetailResponse;

/// Handle requests to retrieve detailed information for a single part.
///
/// **Note: This endpoint is currently unimplemented.**
pub async fn get_part(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
) -> Result<Json<crate::model::responses::base::ApiResponse<PartDetailResponse>>, AppError> {
    unimplemented!()
}
