use serde::{Deserialize, Serialize};

/// Page-based pagination metadata included in list/paginated responses.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponse {
    /// Number of items returned in this page.
    pub limit: u64,

    /// Current page number.
    pub current_page: u64,

    /// Total number of records matching the query.
    pub total_records: u64,

    /// Total number of pages.
    pub total_pages: u64,

    /// Whether there are more items after this page.
    pub has_next: bool,
}

impl PaginationResponse {
    /// Construct page-based pagination metadata.
    ///
    /// - `limit`: the page size that was requested.
    /// - `current_page`: the current page index.
    /// - `total_records`: total matching records (from a COUNT query).
    pub fn new(
        limit: u64,
        current_page: u64,
        total_records: u64,
    ) -> Self {
        let total_pages = if limit == 0 {
            0
        } else {
            (total_records as f64 / limit as f64).ceil() as u64
        };
        
        let has_next = current_page < total_pages;

        Self {
            limit,
            current_page,
            total_records,
            total_pages,
            has_next,
        }
    }
}
