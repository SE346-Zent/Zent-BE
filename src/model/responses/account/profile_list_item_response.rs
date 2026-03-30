use super::prelude::*;
use crate::define_api_response;
use crate::model::responses::common::pagination_meta::PaginationMeta;

#[derive(Deserialize, Debug, Serialize, ToSchema)]
pub struct ProfileListItemResponseData {
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub role_id: i32,
    pub account_status: i32,
}

define_api_response!(ProfileListResponse, Vec<ProfileListItemResponseData>, PaginationMeta);
impl ProfileListResponse {
    pub fn success(
        role: &str,
        data: Vec<ProfileListItemResponseData>,
        pagination: &PaginationQuery,
        total_items: u64,
    ) -> Self {
        Self {
            status_code: 200,
            message: format!("Profiles with role '{}' retrieved successfully", role),
            data,
            meta: PaginationMeta::new(pagination.page, pagination.per_page, total_items),
        }
    }
}
