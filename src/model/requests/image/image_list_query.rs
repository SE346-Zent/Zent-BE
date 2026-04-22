use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ImageListQuery {
    #[serde(rename = "partId")]
    pub part_id: Option<Uuid>,

    #[serde(rename = "productId")]
    pub product_id: Option<Uuid>,
}
