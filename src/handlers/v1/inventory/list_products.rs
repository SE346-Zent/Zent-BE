use axum::{extract::{State, Query}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use crate::model::responses::inventory::product_list_item::ProductListItem;

/// GET /api/v1/inventory/products
pub async fn list_products(
    State(_state): State<AppState>,
    Query(_query): Query<ListProductsQuery>,
) -> Result<Json<crate::model::responses::base::ApiResponse<Vec<ProductListItem>>>, AppError> {
    unimplemented!()
}
