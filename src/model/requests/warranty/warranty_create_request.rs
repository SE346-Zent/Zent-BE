use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct WarrantyCreateRequest {
    pub customer_id: Uuid,
    pub product_id: Uuid,
    pub start_date: String,
    pub end_date: Option<String>,
    pub warranty_status: String,
}
