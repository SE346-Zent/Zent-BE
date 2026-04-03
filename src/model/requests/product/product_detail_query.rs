use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams, Debug)]
pub struct ProductDetailQuery {
    #[serde(rename = "productId")]
    pub product_id: Uuid,
}
