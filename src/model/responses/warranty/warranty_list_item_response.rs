use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


use crate::define_api_response;
use crate::model::responses::common::pagination_meta::PaginationMeta;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct WarrantyListItemResponseData {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub product_id: Uuid,
    pub start_date: String,
    pub end_date: Option<String>,
    pub warranty_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(WarrantyListResponse, Vec<WarrantyListItemResponseData>, PaginationMeta);
impl WarrantyListResponse {
    pub fn success(data: Vec<WarrantyListItemResponseData>, meta: PaginationMeta) -> Self {
        Self {
            status_code: 200,
            message: "Warranty list retrieved successfully".to_string(),
            data,
            meta,
        }
    }
}
