use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderListItem {
    pub id: Uuid,
    pub work_order_num: String,
    pub status: String,
    pub customer_name: String,
    pub product_name: String,
    pub address: String,
    pub appointment: Option<String>,
    pub created_at: String,
}