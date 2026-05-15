use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, IntoParams, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListPartsQuery {
    /// Filter by product model code
    pub model_code: Option<String>,
    /// Filter by part type id
    pub part_type_id: Option<i32>,
    /// Filter by approval status (pending, approved, denied)
    pub approval_status: Option<String>,
    /// Search by part number or serial number
    pub search: Option<String>,
    /// Page number (1-based)
    pub page: Option<u64>,
    /// Items per page
    pub limit: Option<u64>,
    /// Sort field: "created_at" (default), "part_number", "serial_number", "part_type_name"
    pub sort_by: Option<String>,
    /// Sort direction: "asc" (default) or "desc"
    pub sort_order: Option<String>,
}
