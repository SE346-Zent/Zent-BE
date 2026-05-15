use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DenyPartRequest {
    /// Reason why the new part request was denied
    #[validate(length(min = 10, max = 2000))]
    pub reason: String,
}
