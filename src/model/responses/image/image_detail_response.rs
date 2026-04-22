use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


use crate::define_api_response;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ImageDetailResponseData {
    pub id: Uuid,
    pub image_url: String,
    pub part_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub captured_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(ImageDetailResponse, ImageDetailResponseData, Option<()>);
impl ImageDetailResponse {
    pub fn success(data: ImageDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Image retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
