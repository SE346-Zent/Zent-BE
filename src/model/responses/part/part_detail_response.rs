use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::define_api_response;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct PartDetailResponseData {
    pub id: Uuid,
    pub equipment_id: Option<Uuid>,
    pub part_status_id: i32,
    pub customer_id: Uuid,
    pub part_name: String,
    pub quantity: i32,
    pub serial_number: Option<String>,
    pub last_modified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(PartDetailResponse, PartDetailResponseData, Option<()>);
impl PartDetailResponse {
    pub fn success(data: PartDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Part retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
