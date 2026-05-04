use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Validate)]
pub struct LogoutRequest {
    pub refresh_token: String,
}
