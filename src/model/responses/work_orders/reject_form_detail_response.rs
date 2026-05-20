use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RejectFormDetail {
    pub id: Uuid,
    pub approver_id: Option<Uuid>,
    pub approved: bool,
    pub reason: String,
    pub explanation: String,
    /// Object names of photos attached to this rejection form.
    pub photo_urls: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}
