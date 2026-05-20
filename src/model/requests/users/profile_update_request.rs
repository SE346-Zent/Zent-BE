use serde::{Deserialize, Serialize};

/// Request payload for updating the current user's profile.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateRequest {
    /// New full name (optional).
    pub full_name: Option<String>,

    /// New email (optional).
    pub email: Option<String>,

    /// New phone number (optional).
    pub phone: Option<String>,
}
