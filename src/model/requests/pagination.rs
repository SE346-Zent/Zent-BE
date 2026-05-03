use serde::Deserialize;

/// Page-based pagination query parameters for list endpoints.
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
    /// The page number (1-indexed). Default is 1.
    #[serde(default = "default_page")]
    pub page: u64,

    /// Maximum number of items to return (default: 20).
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_page() -> u64 {
    1
}

fn default_limit() -> u64 {
    20
}
