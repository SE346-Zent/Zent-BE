use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RateWorkOrderRequest {
    /// Rating score from 1 to 5
    #[validate(range(min = 1, max = 5, message = "Rating must be between 1 and 5"))]
    pub rating: i32,
    /// Optional review comment from the customer
    #[validate(length(max = 2000, message = "Comment must be at most 2000 characters"))]
    pub comment: Option<String>,
}
