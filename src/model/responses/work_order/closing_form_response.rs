use super::prelude::*;
use crate::define_api_response;

#[derive(Deserialize, Debug, Serialize, utoipa::ToSchema)]
pub struct ClosingFormResponseData {
    pub id: uuid::Uuid,
    pub product_id: uuid::Uuid,
    pub work_order_id: uuid::Uuid,
    pub mtm: String,
    pub serial_number: String,
    pub diagnosis: String,
    pub signature_url: String,
    pub created_at: String,
    pub updated_at: String,
}

define_api_response!(ClosingFormResponse, ClosingFormResponseData, Option<()>);
impl ClosingFormResponse {
    pub fn success(data: ClosingFormResponseData) -> Self {
        Self {
            status_code: 201,
            message: "Closing form created successfully".to_string(),
            data,
            meta: None,
        }
    }
}
