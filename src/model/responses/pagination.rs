use serde::{Deserialize, Serialize};

/// Pagination metadata included in list/paginated responses.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponse {
    /// Current page number (1-based).
    pub current_page: u64,

    /// Number of items per page.
    pub limit: u64,

    /// Total number of records matching the query.
    pub total_records: u64,

    /// Total number of pages.
    pub total_pages: u64,

    /// Whether there is a next page available.
    pub has_next: bool,
}

impl PaginationResponse {
    /// Construct pagination metadata from raw values.
    pub fn new(current_page: u64, limit: u64, total_records: u64) -> Self {
        let total_pages = if limit == 0 {
            0
        } else {
            total_records.div_ceil(limit)
        };

        Self {
            current_page,
            limit,
            total_records,
            total_pages,
            has_next: current_page < total_pages,
        }
    }
}
