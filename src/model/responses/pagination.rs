use serde::{Deserialize, Serialize};

/// Cursor-based pagination metadata included in list/paginated responses.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponse {
    /// Number of items returned in this page.
    pub limit: u64,

    /// Total number of records matching the query.
    pub total_records: u64,

    /// Opaque cursor pointing to the last item of the current page.
    /// Pass this value as `cursor` in the next request to fetch the next page.
    /// `null` when there are no more pages.
    pub next_cursor: Option<String>,

    /// Whether there are more items after this page.
    pub has_next: bool,
}

impl PaginationResponse {
    /// Construct cursor-based pagination metadata.
    ///
    /// - `limit`: the page size that was requested.
    /// - `total_records`: total matching records (from a COUNT query).
    /// - `next_cursor`: the cursor value for the next page, or `None` if this is the last page.
    /// - `has_next`: whether more pages exist.
    pub fn new(
        limit: u64,
        total_records: u64,
        next_cursor: Option<String>,
        has_next: bool,
    ) -> Self {
        Self {
            limit,
            total_records,
            next_cursor,
            has_next,
        }
    }
}
