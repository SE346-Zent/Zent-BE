use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

/// Query parameters for listing and filtering products.
#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListProductsQuery {
    /// Filter by product model code.
    pub model_code: Option<String>,
    /// General search term for product name or serial number.
    pub search: Option<String>,
    /// Page number (1-indexed). Default is 1.
    pub page: Option<u64>,
    /// Maximum number of items per page. Default is 20.
    pub limit: Option<u64>,
    /// Sort field: 'created_at' (default), 'product_name', 'serial_number', 'model_code'.
    pub sort_by: Option<String>,
    /// Sort direction: 'asc' (default) or 'desc'.
    pub sort_order: Option<String>,
}
