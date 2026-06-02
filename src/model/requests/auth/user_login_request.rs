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
    /// Optional FCM token for push notifications.
    pub fcm_token: Option<String>,
    /// Optional device name shown in login history.
    pub device_name: Option<String>,
    /// Optional location label shown in login history.
    pub location: Option<String>,
}

