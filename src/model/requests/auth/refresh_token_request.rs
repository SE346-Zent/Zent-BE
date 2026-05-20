use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request payload for refreshing an access token.
#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    /// The valid refresh token.
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}
