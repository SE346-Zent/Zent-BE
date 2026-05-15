use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListProductsQuery {
    /// Filter by product model code
    pub model_code: Option<String>,
    /// Search by product name or serial number
    pub search: Option<String>,
    /// Page number (1-based)
    pub page: Option<u64>,
    /// Items per page
    pub limit: Option<u64>,
    /// Sort field: "created_at" (default), "product_name", "serial_number", "model_code"
    pub sort_by: Option<String>,
    /// Sort direction: "asc" (default) or "desc"
    pub sort_order: Option<String>,
}
