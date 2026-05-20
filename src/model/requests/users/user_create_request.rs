use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for creating a new user by an admin or super admin.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserCreateRequest {
    /// The role ID to assign to the new user (e.g., Technician or Admin).
    pub role_id: i32,

    /// User's full name.
    #[validate(length(min = 1, message = "Full name is required"))]
    pub full_name: String,

    /// User's email address.
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// User's phone number (optional).
    pub phone: Option<String>,

    /// Optional password for the new user. If `None` and `generate_password` is true,
    /// a random password will be generated.
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: Option<String>,

    /// Whether to auto-generate a password if none is provided.
    #[serde(default)]
    pub generate_password: Option<bool>,

    /// The province to assign. Required when SuperAdmin creates an Admin;
    /// overridden by the caller's province when Admin creates a Technician.
    pub province: Option<String>,
}
