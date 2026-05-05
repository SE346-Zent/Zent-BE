use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::model::responses::pagination::PaginationResponse;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderListItem {
    pub id: Uuid,
    pub work_order_number: String,
    pub status: String,
    pub customer_name: String,
    pub product_name: String,
    pub symptom_name: String,
    pub city: String,
    pub province: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderListResponse {
    pub data: Vec<WorkOrderListItem>,
    pub pagination: PaginationResponse,
}
