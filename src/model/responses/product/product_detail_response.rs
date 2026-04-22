use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::define_api_response;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ProductDetailResponseData {
    pub id: Uuid,
    pub product_model_code: String,
    pub customer_id: Uuid,
    pub serial_number: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(ProductDetailResponse, ProductDetailResponseData, Option<()>);
impl ProductDetailResponse {
    pub fn success(data: ProductDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Product retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
