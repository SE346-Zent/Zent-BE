use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComplaintWorkOrderRequest {
    /// Complaint message from the customer
    #[validate(length(min = 1, max = 2000, message = "Complaint message must be between 1 and 2000 characters"))]
    pub message: String,
}
