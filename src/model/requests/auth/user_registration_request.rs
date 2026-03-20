use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UserRegistrationRequest {
    #[validate(length(min = 1, message = "Full name is required"))]
    pub full_name: String,

    // TODO: Implement more sophisticated email validation; validator cannot catch special, legal email formats
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,

    // Optional parameter but treated as required. Basic non-empty check
    #[validate(length(min = 1, message = "Phone number is required"))]
    pub phone_number: String,
}
