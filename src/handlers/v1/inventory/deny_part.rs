use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::deny_part_request::DenyPartRequest;

/// POST /api/v1/inventory/parts/{id}/deny
pub async fn deny_part(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
    Json(_payload): Json<DenyPartRequest>,
) -> Result<Json<crate::model::responses::base::ApiResponse<()>>, AppError> {
    unimplemented!()
}
