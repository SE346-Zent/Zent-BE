use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfileUpdateRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserCreateRequest {
    pub role_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub password: Option<String>,
    pub generate_password: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserStatusUpdateRequest {
    pub account_status_id: i32,
}
