use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponseData {
    pub id: i32,
    pub role_id: i32,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub account_status_id: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponseData {
    pub users: Vec<UserResponseData>,
    pub total: u64,
}
