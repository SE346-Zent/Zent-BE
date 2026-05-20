use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Public-facing user data returned by admin endpoints.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserResponseData {
    /// Unique user identifier.
    pub id: Uuid,

    /// Role ID of the user.
    pub role_id: i32,

    /// Full name.
    pub full_name: String,

    /// Email address.
    pub email: String,

    /// Phone number (optional).
    pub phone: Option<String>,

    /// Province the user belongs to (optional — Customers may not have one).
    pub province: Option<String>,

    /// Account status ID.
    pub account_status_id: i32,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}
