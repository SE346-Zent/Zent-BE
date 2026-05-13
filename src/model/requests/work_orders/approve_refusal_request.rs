use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ApproveRefusalRequest {
    pub technician_id: Uuid,
}
