use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for user login.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate)]
pub struct UserLoginRequest {
    /// User's email address.
    #[validate(email)]
    pub email: String,
    /// User's password.
    #[validate(length(min = 1))]
    pub password: String,
}
