use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct CreateWorkOrderRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub role: String,
    pub work_order_status_id: i32,
    pub country: String,
    pub state: String,
    pub city: String,
    pub address: String,
    pub building: Option<String>,
    pub appointment: String,
    pub reference_ticket_id: Option<Uuid>,
    pub admin_id: Uuid,
    pub customer_id: Uuid,
    pub technician_id: Option<Uuid>,
    pub complete_form_id: Option<Uuid>,
    pub work_order_symptom_id: i32,
    pub product_id: Uuid,
}
