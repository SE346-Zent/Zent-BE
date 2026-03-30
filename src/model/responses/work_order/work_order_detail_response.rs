use super::prelude::*;
use crate::define_api_response;

#[derive(Deserialize, Debug, Serialize, utoipa::ToSchema)]
pub struct WorkOrderDetailResponseData {
    pub work_order_id: uuid::Uuid,
    pub title: String,
    pub address_string: String,
    pub description: String,
    pub status_id: i32,
    pub priority: i32,
    pub reject_reason: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub version: i32,
    pub admin_id: uuid::Uuid,
    pub customer_id: uuid::Uuid,
    pub technician_id: uuid::Uuid,
}

define_api_response!(WorkOrderDetailResponse, WorkOrderDetailResponseData, Option<()>);
impl WorkOrderDetailResponse {
    pub fn success(data: WorkOrderDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Work order retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
