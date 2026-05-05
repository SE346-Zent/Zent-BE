use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate, Clone)]
pub struct CreateWorkOrderRequest {
    pub product_id: Uuid,
    pub work_order_symptom_id: i32,
    pub reference_ticket_id: Option<Uuid>,
    #[validate(length(min = 1, max = 1000))]
    pub description: String,
    pub appointment: DateTime<Utc>,
    #[validate(length(min = 1, max = 255))]
    pub first_name: String,
    #[validate(length(min = 1, max = 255))]
    pub last_name: String,
    #[validate(email)]
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub country: String,
    pub province: String,
    pub city: String,
    pub address: String,
    pub building: Option<String>,
}
