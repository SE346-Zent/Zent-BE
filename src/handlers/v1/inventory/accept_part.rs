use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;

/// Handle requests to accept a part registration form.
///
/// **Note: This endpoint is currently unimplemented.**
pub async fn accept_part(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
) -> Result<Json<crate::model::responses::base::ApiResponse<()>>, AppError> {
    unimplemented!()
}
