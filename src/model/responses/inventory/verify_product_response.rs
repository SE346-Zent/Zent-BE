use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VerifyProductResponse {
    pub product_id: Uuid,
    pub serial_number: String,
    pub product_name: String,
    pub product_model_code: String,
    pub is_registered: bool,
    pub warranty_status: String,
    pub warranty_start_date: Option<String>,
    pub warranty_end_date: Option<String>,
    pub message: String,
}
