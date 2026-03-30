use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateClosingFormRequest {
    pub work_order_counting: String,
    pub mtm: String,
    pub serial_number: String,
    pub diagnosis: String,
}
