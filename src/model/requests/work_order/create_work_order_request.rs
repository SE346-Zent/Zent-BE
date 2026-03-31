use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateWorkOrderRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone_number: String,
    pub role: String,
    pub work_order_status_id: i32,
    pub country: String,
    pub state: String,
    pub city: String,
    pub address: String,
    pub building: String,
    pub appointment: String,
    pub reference_ticket: String,
    pub admin_id: Uuid,
    pub customer_id: Uuid,
    pub technician_id: Uuid,
    pub complete_form_id: Uuid,
    pub reject_reason: String,
}
