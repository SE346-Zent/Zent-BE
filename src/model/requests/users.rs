use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfileUpdateRequest {
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserCreateRequest {
    pub role_id: i32,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub password: Option<String>,
    pub generate_password: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserStatusUpdateRequest {
    pub account_status_id: i32,
}
