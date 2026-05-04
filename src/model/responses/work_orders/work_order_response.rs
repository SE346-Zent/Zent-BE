use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrderResponseData {
    pub id: Uuid,
    pub work_order_number: String,
    pub status: String,
}
