use serde::{Deserialize, Serialize};

/// Reusable pagination metadata included in every list response.
#[derive(Debug, Serialize, utoipa::ToSchema, Deserialize)]
pub struct PaginationMeta {
    /// Current page number (1-based).
    pub page: u64,
    /// Number of items per page.
    pub per_page: u64,
    /// Total number of items matching the query.
    pub total_items: u64,
    /// Total number of pages.
    pub total_pages: u64,
}

impl PaginationMeta {
    pub fn new(page: u64, per_page: u64, total_items: u64) -> Self {
        let total_pages = total_items.div_ceil(per_page);
        Self {
            page,
            per_page,
            total_items,
            total_pages,
        }
    }
}

/// Reusable pagination query parameters.
/// Embed this inside a specific endpoint's query struct via `#[serde(flatten)]`.
///
/// Example:
/// ```rust, ignore
/// #[derive(Deserialize, utoipa::IntoParams)]
/// pub struct ProfileListQuery {
///     pub role: String,
///     #[serde(flatten)]
///     pub pagination: PaginationQuery,
/// }
/// ```
#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct PaginationQuery {
    /// Page number (default: 1)
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page (default: 20)
    #[serde(default = "default_per_page")]
    pub per_page: u64,
}

impl PaginationQuery {
    /// Calculates the offset for a database query.
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.per_page
    }
}

fn default_page() -> u64 {
    1
}

fn default_per_page() -> u64 {
    20
}
