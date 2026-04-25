use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResendOtpRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}
