use super::prelude::*;
use crate::define_api_response;

#[derive(Deserialize, Debug, Serialize, ToSchema)]
pub struct ProfileDetailResponseData {
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub role_id: i32,
    pub account_status: i32,
}

define_api_response!(ProfileDetailResponse, ProfileDetailResponseData, Option<()>);
impl ProfileDetailResponse {
    pub fn success(data: ProfileDetailResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Profile retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}
