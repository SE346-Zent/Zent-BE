use axum::{extract::{State, Query}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::part_list_item::PartListItem;

/// GET /api/v1/inventory/parts
pub async fn list_parts(
    State(_state): State<AppState>,
    Query(_query): Query<ListPartsQuery>,
) -> Result<Json<crate::model::responses::base::ApiResponse<Vec<PartListItem>>>, AppError> {
    unimplemented!()
}
