use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;

/// POST /api/v1/inventory/parts/{id}/accept
pub async fn accept_part(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
) -> Result<Json<crate::model::responses::base::ApiResponse<()>>, AppError> {
    unimplemented!()
}
