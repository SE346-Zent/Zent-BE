use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckSerialRequest {
    #[validate(length(min = 1, max = 255))]
    pub serial_number: String,
}
