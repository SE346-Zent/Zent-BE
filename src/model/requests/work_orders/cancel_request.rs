use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelWorkOrderRequest {
    /// Reason for cancellation (provided by the customer)
    #[validate(length(min = 1, max = 1000))]
    pub reason: String,
    /// Additional comments or explanation (optional)
    #[validate(length(min = 0, max = 2000))]
    #[serde(default)]
    pub additional_comments: Option<String>,
}
