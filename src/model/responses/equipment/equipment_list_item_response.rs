use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::define_api_response;
use crate::model::responses::common::pagination_meta::PaginationMeta;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct EquipmentListItemResponseData {
    pub id: Uuid,
    pub equipment_status_id: i32,
    pub model_id: i32,
    pub customer_id: Uuid,
    pub equipment_name: String,
    pub serial_number: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(EquipmentListResponse, Vec<EquipmentListItemResponseData>, PaginationMeta);
impl EquipmentListResponse {
    pub fn success(data: Vec<EquipmentListItemResponseData>, meta: PaginationMeta) -> Self {
        Self {
            status_code: 200,
            message: "Equipment list retrieved successfully".to_string(),
            data,
            meta,
        }
    }
}
