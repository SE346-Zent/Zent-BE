use axum::extract::FromRef;
use std::sync::Arc;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

#[derive(Clone)]
pub struct AppState {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub db: DatabaseConnection,
    pub rabbitmq: Arc<lapin::Connection>,
    pub access_token_ttl: AccessTokenDefaultTTLSeconds,
    pub session_ttl: SessionDefaultTTLSeconds,
}

impl AppState {
    pub fn new(
        secret: &[u8],
        db: DatabaseConnection,
        rabbitmq: Arc<lapin::Connection>,
        access_token_ttl: i64,
        session_ttl: i64,
    ) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            encoding_key: EncodingKey::from_secret(secret),
            db,
            rabbitmq,
            access_token_ttl: AccessTokenDefaultTTLSeconds(access_token_ttl),
            session_ttl: SessionDefaultTTLSeconds(session_ttl),
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

impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for AccessTokenDefaultTTLSeconds {
    fn from_ref(state: &AppState) -> Self {
        state.access_token_ttl
    }
}

impl FromRef<AppState> for SessionDefaultTTLSeconds {
    fn from_ref(state: &AppState) -> Self {
        state.session_ttl
    }
}

impl FromRef<AppState> for Arc<lapin::Connection> {
    fn from_ref(state: &AppState) -> Self {
        state.rabbitmq.clone()
    }
}
