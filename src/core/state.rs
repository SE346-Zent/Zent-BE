use axum::extract::FromRef;
use std::collections::HashMap;
use std::sync::Arc;
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::database::DatabaseManager;
use crate::infrastructure::cache::ValkeyManager;
use crate::infrastructure::mq::RabbitMQManager;
use crate::services::v1::auth::AuthService;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

#[derive(Clone)]
pub struct AppState {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub db: Arc<DatabaseManager>,
    pub valkey: Arc<ValkeyManager>,
    pub rabbitmq: Arc<RabbitMQManager>,
    pub access_token_ttl: AccessTokenDefaultTTLSeconds,
    pub session_ttl: SessionDefaultTTLSeconds,
    pub lookup_tables: Arc<LookupTables>,
    pub templates: Arc<HashMap<String, String>>,
    pub auth_service: Arc<AuthService>,
}

impl AppState {
    pub fn new(
        secret: &[u8],
        db: Arc<DatabaseManager>,
        valkey: Arc<ValkeyManager>,
        rabbitmq: Arc<RabbitMQManager>,
        access_token_ttl: i64,
        session_ttl: i64,
        lookup_tables: LookupTables,
        templates: HashMap<String, String>,
        auth_service: AuthService,
    ) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            encoding_key: EncodingKey::from_secret(secret),
            db,
            valkey,
            rabbitmq,
            access_token_ttl: AccessTokenDefaultTTLSeconds(access_token_ttl),
            session_ttl: SessionDefaultTTLSeconds(session_ttl),
            lookup_tables: Arc::new(lookup_tables),
            templates: Arc::new(templates),
            auth_service: Arc::new(auth_service),
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

impl FromRef<AppState> for Arc<DatabaseManager> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<ValkeyManager> {
    fn from_ref(state: &AppState) -> Self {
        state.valkey.clone()
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

impl FromRef<AppState> for Arc<RabbitMQManager> {
    fn from_ref(state: &AppState) -> Self {
        state.rabbitmq.clone()
    }
}

impl FromRef<AppState> for Arc<LookupTables> {
    fn from_ref(state: &AppState) -> Self {
        state.lookup_tables.clone()
    }
}

impl FromRef<AppState> for Arc<HashMap<String, String>> {
    fn from_ref(state: &AppState) -> Self {
        // Default to templates for this type if requested via FromRef
        state.templates.clone()
    }
}

impl FromRef<AppState> for Arc<AuthService> {
    fn from_ref(state: &AppState) -> Self {
        state.auth_service.clone()
    }
}
