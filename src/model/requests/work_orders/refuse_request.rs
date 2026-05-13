use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefuseWorkOrderRequest {
    #[validate(length(min = 1, max = 255))]
    pub reason: String,
    #[validate(length(max = 1000))]
    pub explanation: String,
    #[validate(length(min = 0, max = 5))]
    pub evidence_image_urls: Vec<String>,
}

#[derive(ToSchema)]
pub struct RefuseWorkOrderMultipart {
    pub reason: String,
    pub explanation: String,
    #[schema(value_type = Vec<String>, format = Binary)]
    pub photos: Vec<String>,
}
