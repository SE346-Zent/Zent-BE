use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CancelWorkOrderRequest {
    /// Optional reason for cancellation (provided by the customer)
    #[validate(length(min = 0, max = 1000))]
    #[serde(default)]
    pub reason: Option<String>,
}
