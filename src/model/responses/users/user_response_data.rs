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

    /// Employee ID (optional).
    pub employee_id: Option<String>,

    /// Rating counts (1-5) for technicians (optional).
    pub rating_counts: Option<std::collections::HashMap<String, i64>>,

    /// Average rating for technicians (optional, 0.0–5.0).
    pub average_rating: Option<f64>,

    /// Number of active work orders currently assigned to this technician (optional).
    pub workload: Option<i64>,

    /// Avatar image object name for PAR read (optional).
    pub avatar_image_name: Option<String>,

    /// ISO-8601 creation timestamp.
    pub created_at: String,

    /// ISO-8601 last-update timestamp.
    pub updated_at: String,
}
