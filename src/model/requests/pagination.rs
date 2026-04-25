use serde::Deserialize;

/// Cursor-based pagination query parameters for list endpoints.
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
    /// Opaque cursor pointing to the last item of the previous page.
    /// Pass `null` or omit for the first request.
    pub cursor: Option<String>,

    /// Maximum number of items to return (default: 20).
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    20
}
