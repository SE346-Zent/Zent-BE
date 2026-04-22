use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateClosingFormRequest {
    pub product_id: Uuid,
    pub work_order_id: Uuid,
    pub mtm: String,
    pub serial_number: String,
    pub diagnosis: String,
    pub signature_url: String,
}
