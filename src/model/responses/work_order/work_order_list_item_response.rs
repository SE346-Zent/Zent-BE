use super::prelude::*;
use crate::define_api_response;
use crate::model::responses::common::pagination_meta::PaginationMeta;

#[derive(Deserialize, Debug, Serialize, utoipa::ToSchema)]
pub struct WorkOrderListItemResponseData {
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
    pub deleted_at: Option<String>,
    pub admin_id: uuid::Uuid,
    pub customer_id: uuid::Uuid,
    pub technician_id: uuid::Uuid,
    pub complete_form_id: uuid::Uuid,
    pub reject_reason: String,
    pub product_id: uuid::Uuid,
    pub work_order_symptom_id: i32,
}

define_api_response!(WorkOrderListResponse, Vec<WorkOrderListItemResponseData>, PaginationMeta);
impl WorkOrderListResponse {
    pub fn success(data: Vec<WorkOrderListItemResponseData>, meta: PaginationMeta) -> Self {
        Self {
            status_code: 200,
            message: "Work orders retrieved successfully".to_string(),
            data,
            meta,
        }
    }
}
