use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Response payload for the "get me" endpoint.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeResponseData {
    /// Unique user identifier.
    pub id: Uuid,

    /// Role ID.
    pub role_id: i32,

    /// Full name.
    pub full_name: String,

    /// Email address.
    pub email: String,

    /// Phone number (optional).
    pub phone: Option<String>,

    /// Province (optional).
    pub province: Option<String>,

    /// Account status ID.
    pub account_status_id: i32,

    /// Employee ID (optional).
    pub employee_id: Option<String>,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}
