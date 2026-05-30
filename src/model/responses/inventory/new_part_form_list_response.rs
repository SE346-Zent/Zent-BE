use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPartFormListItem {
    pub id: Uuid,
    pub part_number: String,
    pub part_type_name: String,
    pub work_order_number: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPartFormStatusSummary {
    pub pending: u64,
    pub approved: u64,
    pub rejected: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPartFormListResponse {
    pub items: Vec<NewPartFormListItem>,
    pub summary: NewPartFormStatusSummary,
}