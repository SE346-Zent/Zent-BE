use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Detailed view of a product including its parts.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductDetailResponse {
    pub product_id: Uuid,
    pub product_name: String,
    pub model_code: String,
    pub model_name: String,
    pub serial_number: String,
    /// Customer who registered/owns this product
    pub customer_id: Uuid,
    pub customer_name: String,
    /// Parts installed in this product
    pub parts: Vec<super::part_list_item::PartListItem>,
    pub created_at: String,
    pub updated_at: String,
}
