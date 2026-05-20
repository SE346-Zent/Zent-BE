use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for user logout.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate)]
pub struct LogoutRequest {
    /// The refresh token to be invalidated.
    pub refresh_token: String,
}
