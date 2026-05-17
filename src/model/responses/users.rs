use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponseData {
    pub id: Uuid,
    pub role_id: i32,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub account_status_id: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponseData {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponseData {
    pub users: Vec<UserResponseData>,
    pub total: u64,
}
