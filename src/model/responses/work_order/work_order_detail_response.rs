use super::prelude::*;
use crate::define_api_response;

#[derive(Deserialize, Debug, Serialize, utoipa::ToSchema)]
pub struct WorkOrderDetailResponseData {
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone_number: String,
    pub work_order_status_id: i32,
    pub country: String,
    pub state: String,
    pub city: String,
    pub address: String,
    pub building: String,
    pub appointment: String,
    pub reference_ticket: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: String,
    pub admin_id: uuid::Uuid,
    pub customer_id: uuid::Uuid,
    pub technician_id: uuid::Uuid,
    pub complete_form_id: uuid::Uuid,
    pub reject_reason: String,
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
