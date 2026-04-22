use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


use crate::define_api_response;
use crate::model::responses::common::pagination_meta::PaginationMeta;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ImageListItemResponseData {
    pub id: Uuid,
    pub image_url: String,
    pub part_id: Option<Uuid>,
    pub product_id: Option<Uuid>,
    pub captured_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(ImageListResponse, Vec<ImageListItemResponseData>, PaginationMeta);
impl ImageListResponse {
    pub fn success(data: Vec<ImageListItemResponseData>, meta: PaginationMeta) -> Self {
        Self {
            status_code: 200,
            message: "Image list retrieved successfully".to_string(),
            data,
            meta,
        }
    }
}
