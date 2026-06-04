use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A product registered by the customer, enriched with model image and warranty info.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyProductItem {
    /// Product ID (from SCM).
    pub product_id: Uuid,
    /// Product name.
    pub product_name: String,
    /// Product model code.
    pub product_model_code: String,
    /// Serial number.
    pub serial_number: String,
    /// Product model image URL (from SCM product model).
    pub image_url: Option<String>,
    /// Warranty details, if a warranty record exists for this product.
    pub warranty: Option<MyProductWarranty>,
}

/// Warranty information for a customer's product.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyProductWarranty {
    pub id: Uuid,
    pub start_date: String,
    pub end_date: String,
    pub warranty_status: String,
    pub days_remaining: i64,
}
