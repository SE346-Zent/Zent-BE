use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::register_product_request::RegisterProductRequest;
use crate::model::responses::inventory::register_product_response::RegisterProductResponse;

/// Handle requests to register a new product by a customer.
///
/// **Note: This endpoint is currently unimplemented.**
pub async fn register_product(
    State(_state): State<AppState>,
    Json(_payload): Json<RegisterProductRequest>,
) -> Result<Json<crate::model::responses::base::ApiResponse<RegisterProductResponse>>, AppError> {
    unimplemented!()
}
