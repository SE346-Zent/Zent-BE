use axum::extract::FromRef;
use jsonwebtoken::{DecodingKey, EncodingKey};

#[derive(Clone)]
pub struct AppState {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
}

impl AppState {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            encoding_key: EncodingKey::from_secret(secret),
        }
    }
}

impl FromRef<AppState> for DecodingKey {
    fn from_ref(state: &AppState) -> Self {
        state.decoding_key.clone()
    }
}

impl FromRef<AppState> for EncodingKey {
    fn from_ref(state: &AppState) -> Self {
        state.encoding_key.clone()
    }
}
