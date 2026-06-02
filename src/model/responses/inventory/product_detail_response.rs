use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Warranty summary shown on the product detail screen.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductWarrantySummary {
    pub warranty_status: String,
    pub support_status: String,
    pub support_days_remaining: i64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// Compact work order entry shown in the product detail history list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductWorkOrderHistoryItem {
    pub work_order_id: Uuid,
    pub work_order_number: String,
    pub status: String,
    pub date: String,
}

/// Detailed view of a product including its parts.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProductDetailResponse {
    pub product_id: Uuid,
    pub title: String,
    pub model_code: String,
    pub model_name: String,
    pub product_image_url: Option<String>,
    pub serial_number: String,
    pub warranty: Option<ProductWarrantySummary>,
    pub work_order_history: Vec<ProductWorkOrderHistoryItem>,
    pub created_at: String,
    pub updated_at: String,
}
