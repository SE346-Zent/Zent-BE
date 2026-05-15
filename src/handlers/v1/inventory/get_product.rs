use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::responses::inventory::product_detail_response::ProductDetailResponse;

/// GET /api/v1/inventory/products/{id}
pub async fn get_product(
    State(_state): State<AppState>,
    Path(_id): Path<uuid::Uuid>,
) -> Result<Json<crate::model::responses::base::ApiResponse<ProductDetailResponse>>, AppError> {
    unimplemented!()
}
