use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RejectFormListItem {
    pub reject_form_id: Uuid,
    pub work_order_id: Uuid,
    pub work_order_number: String,
    pub technician_name: String,
    pub customer_name: String,
    pub reason: String,
    pub approved: bool,
    pub created_at: Option<DateTime<Utc>>,
}
