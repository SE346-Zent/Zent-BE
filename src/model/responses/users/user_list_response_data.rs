use serde::{Deserialize, Serialize};

use super::user_response_data::UserResponseData;

/// Response payload for listing users.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserListResponseData {
    /// The list of user records.
    pub users: Vec<UserResponseData>,

    /// Total number of matching records (for pagination).
    pub total: u64,
}
