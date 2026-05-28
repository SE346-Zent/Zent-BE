use utoipa::IntoParams;

pub mod get_me;
pub mod update_me;
pub mod close_account;
pub mod list_users;
pub mod get_user;
pub mod create_user;
pub mod update_status;

/// Query parameters for the list users endpoint.
#[derive(Debug, serde::Deserialize, IntoParams)]
pub struct UserListQuery {
    /// Page number (1-indexed). Default: 1.
    pub page: Option<u64>,
    /// Number of items per page. Default: 20.
    pub page_size: Option<u64>,
    /// Filter by role name (e.g., "technician", "admin"). SuperAdmin only.
    pub role: Option<String>,
}
