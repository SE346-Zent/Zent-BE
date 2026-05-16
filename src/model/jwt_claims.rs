use serde::{Deserialize, Serialize};

/// JWT Claims structure representing the payload of an authentication token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (User ID)
    pub sub: String,
    /// Issued At (Timestamp)
    pub iat: usize,
    /// Expiration Time (Timestamp)
    pub exp: usize,
}
