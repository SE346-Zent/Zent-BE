use axum::{extract::{State, Query}, Json};
use crate::core::state::AppState;
use crate::core::errors::AppError;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::part_list_item::PartListItem;

/// Handle requests to retrieve a paginated list of parts.
///
/// **Note: This endpoint is currently unimplemented.**
pub async fn list_parts(
    State(_state): State<AppState>,
    Query(_query): Query<ListPartsQuery>,
) -> Result<Json<crate::model::responses::base::ApiResponse<Vec<PartListItem>>>, AppError> {
    unimplemented!()
}
