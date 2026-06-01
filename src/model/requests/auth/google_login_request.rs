use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for logging in via Google/Firebase.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GoogleLoginRequest {
    /// The ID token obtained from Google or Firebase client SDKs.
    #[validate(length(min = 1, message = "ID token cannot be empty"))]
    pub id_token: String,
    /// Optional FCM token for push notifications.
    pub fcm_token: Option<String>,
    /// Optional device name shown in login history.
    pub device_name: Option<String>,
    /// Optional location label shown in login history.
    pub location: Option<String>,
}

