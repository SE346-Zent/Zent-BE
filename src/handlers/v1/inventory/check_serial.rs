use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::check_serial_request::CheckSerialRequest;

/// Handle requests to validate if a product serial number exists in the catalog.
///
/// **Note: This endpoint is currently unimplemented.**
pub async fn check_serial(
    State(_state): State<AppState>,
    Json(_payload): Json<CheckSerialRequest>,
) -> Result<Json<crate::model::responses::base::ApiResponse<bool>>, AppError> {
    unimplemented!()
}
