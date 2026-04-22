use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ImageCreateRequest {
    pub image_url: String,
    pub part_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub captured_at: String,
}
