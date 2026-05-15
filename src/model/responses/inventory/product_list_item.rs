use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Summary view of a product for list queries.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductListItem {
    pub product_id: Uuid,
    pub product_name: String,
    pub model_code: String,
    pub serial_number: String,
    /// Number of parts installed in this product
    pub part_count: i64,
    pub created_at: String,
}
