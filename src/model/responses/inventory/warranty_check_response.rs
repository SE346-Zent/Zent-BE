use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WarrantyCheckResponse {
    pub product_id: Uuid,
    pub serial_number: String,
    pub product_name: String,
    /// Status: "active", "expired", or "none"
    pub warranty_status: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
