use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub message: String,
    pub data: Option<Value>,
    pub meta: Option<Value>,
}

impl RegisterResponse {
    pub fn success(message: &str) -> Self {
        Self {
            status_code: 201,
            message: message.to_string(),
            data: None,
            meta: None,
        }
    }
}
