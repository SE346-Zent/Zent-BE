use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PauseWorkOrderRequest {
    /// Reason for pausing (required, min 10 chars)
    #[validate(length(min = 10, message = "Reason must be at least 10 characters"))]
    pub reason: String,
    /// Additional explanation (optional)
    pub explanation: Option<String>,
}
