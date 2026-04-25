use serde::Deserialize;

/// Pagination query parameters for list endpoints.
///
/// Embed via `#[serde(flatten)]` inside endpoint-specific query structs:
/// ```rust,ignore
/// #[derive(Deserialize, utoipa::IntoParams)]
/// pub struct ListItemsQuery {
///     pub filter: Option<String>,
///     #[serde(flatten)]
///     pub pagination: PaginationRequest,
/// }
/// ```
#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct PaginationRequest {
    /// Page number (default: 1).
    #[serde(default = "default_page")]
    pub page: u64,

    /// Items per page (default: 20).
    #[serde(default = "default_limit")]
    pub limit: u64,
}

impl PaginationRequest {
    /// Compute the SQL OFFSET value for the current page.
    pub fn offset(&self) -> u64 {
        (self.page.saturating_sub(1)) * self.limit
    }
}

fn default_page() -> u64 {
    1
}

fn default_limit() -> u64 {
    20
}
