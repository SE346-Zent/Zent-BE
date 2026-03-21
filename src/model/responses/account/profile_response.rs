use super::prelude::*;
use crate::define_api_response;


// ---------------------------------------------------------------------------
// Response data
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug, Serialize, ToSchema)]
pub struct ProfileResponseData {
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub role_id: i32,
    pub account_status: i32,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

// Single profile response
define_api_response!(ProfileResponse, ProfileResponseData, Option<()>);
impl ProfileResponse {
    pub fn success(data: ProfileResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Profile retrieved successfully".to_string(),
            data,
            meta: None,
        }
    }
}

// Role-filtered, paginated list of profiles response
define_api_response!(ProfileListResponse, Vec<ProfileResponseData>, PaginationMeta);
impl ProfileListResponse {
    pub fn success(
        role: &str,
        data: Vec<ProfileResponseData>,
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