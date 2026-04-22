use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct WarrantyListQuery {
    #[serde(rename = "productId")]
    pub product_id: Option<Uuid>,

    #[serde(rename = "customerId")]
    pub customer_id: Option<Uuid>,
}
