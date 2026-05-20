use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for new user registration.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserRegistrationRequest {
    /// User's full name.
    #[validate(length(min = 1, message = "Full name is required"))]
    pub full_name: String,

    /// User's email address.
    // TODO: Implement more sophisticated email validation; validator cannot catch special, legal email formats
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    /// User's password (minimum 6 characters).
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,

    /// User's phone number.
    // Optional parameter but treated as required. Basic non-empty check
    #[validate(length(min = 1, message = "Phone number is required"))]
    pub phone_number: String,
}
