use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;


use crate::define_api_response;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct WarrantyDetailResponseData {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub equipment_id: Uuid,
    pub start_date: String,
    pub end_date: Option<String>,
    pub warranty_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

define_api_response!(WarrantyDetailResponse, WarrantyDetailResponseData, Option<()>);
impl WarrantyDetailResponse {
    pub fn success(data: WarrantyDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Warranty retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
