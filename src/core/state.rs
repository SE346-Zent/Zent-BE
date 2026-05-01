use axum::extract::FromRef;
use std::collections::HashMap;
use std::sync::Arc;
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::database::DatabasePool;
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::mq::RabbitMQClient;
use crate::services::v1::auth::AuthService;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

/// AppState acts as a **ServiceRegistry**: it only holds JWT keys,
/// lookup tables, and service instances. Infrastructure concerns
/// (database, cache, message queue) are owned by the individual
/// services that need them.
#[derive(Clone)]
pub struct AppState {
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub lookup_tables: Arc<LookupTables>,
    pub auth_service: Arc<AuthService>,
}

impl AppState {
    /// Constructor kept with the original 9-parameter signature so that
    /// existing integration tests compile without modification.
    /// Infrastructure parameters are accepted but not stored — they
    /// are already owned by the services themselves.
    pub fn new(
        secret: &[u8],
        _db: Arc<DatabasePool>,
        _valkey: Arc<ValkeyClient>,
        _rabbitmq: Arc<RabbitMQClient>,
        _access_token_ttl: i64,
        _session_ttl: i64,
        lookup_tables: LookupTables,
        _templates: HashMap<String, String>,
        auth_service: AuthService,
    ) -> Self {
        Self {
            decoding_key: DecodingKey::from_secret(secret),
            encoding_key: EncodingKey::from_secret(secret),
            lookup_tables: Arc::new(lookup_tables),
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

impl FromRef<AppState> for Arc<LookupTables> {
    fn from_ref(state: &AppState) -> Self {
        state.lookup_tables.clone()
    }
}

impl FromRef<AppState> for Arc<AuthService> {
    fn from_ref(state: &AppState) -> Self {
        state.auth_service.clone()
    }
}
