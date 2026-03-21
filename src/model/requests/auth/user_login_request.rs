use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLoginRequest {
    pub email: String,
    pub password: String,
}
