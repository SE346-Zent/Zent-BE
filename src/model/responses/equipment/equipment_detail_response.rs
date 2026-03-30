use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::define_api_response;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct EquipmentDetailResponseData {
    pub id: Uuid, // TODO: Wait, in the migration id was Uuid for Equipments right? I think so.
    pub equipment_status_id: i32,
    pub model_id: i32,
    pub customer_id: Uuid,
    pub equipment_name: String,
    pub serial_number: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(EquipmentDetailResponse, EquipmentDetailResponseData, Option<()>);
impl EquipmentDetailResponse {
    pub fn success(data: EquipmentDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Equipment retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
