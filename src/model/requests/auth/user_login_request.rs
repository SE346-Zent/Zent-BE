use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserLoginRequest {
    pub email: String,
    pub password: String,
}
