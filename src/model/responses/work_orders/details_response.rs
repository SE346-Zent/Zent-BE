use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderDetails {
    pub id: Uuid,
    pub work_order_number: String,
    pub status: String,
    pub customer_id: Uuid,
    pub customer_name: String,
    pub product_id: Uuid,
    pub product_name: String,
    pub reference_ticket_id: Option<Uuid>,
    pub symptom_name: String,
    pub description: String,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub country: String,
    pub province: String,
    pub city: String,
    pub address: String,
    pub building: Option<String>,
    pub appointment: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
