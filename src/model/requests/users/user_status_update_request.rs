use serde::{Deserialize, Serialize};

/// Request payload for updating a user's account status.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusUpdateRequest {
    /// The new account status ID to set.
    pub account_status_id: i32,
}
