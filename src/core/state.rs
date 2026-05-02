use axum::extract::FromRef;
use std::sync::Arc;
use std::collections::HashMap;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;
use lapin::Connection;

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

/// AppState now holds infrastructure resources (db, cache, mq) directly.
/// This enables Handlers to act as Orchestrators by accessing infrastructure
/// directly while calling pure logic from stateless service modules.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub valkey: Option<Arc<ValkeyClient>>,
    pub rabbitmq: Option<Arc<Connection>>,
    pub templates: Arc<HashMap<String, String>>,
    pub access_token_ttl: AccessTokenDefaultTTLSeconds,
    pub session_ttl: SessionDefaultTTLSeconds,
    pub decoding_key: DecodingKey,
    pub encoding_key: EncodingKey,
    pub lookup_tables: Arc<LookupTables>,
}

impl AppState {
    pub fn new(
        secret: &[u8],
        lookup_tables: LookupTables,
        db: DatabaseConnection,
        valkey: Option<Arc<ValkeyClient>>,
        rabbitmq: Option<Arc<Connection>>,
        templates: HashMap<String, String>,
        access_token_ttl: AccessTokenDefaultTTLSeconds,
        session_ttl: SessionDefaultTTLSeconds,
    ) -> Self {
        Self {
            db: Arc::new(db),
            valkey,
            rabbitmq,
            templates: Arc::new(templates),
            access_token_ttl,
            session_ttl,
            decoding_key: DecodingKey::from_secret(secret),
            encoding_key: EncodingKey::from_secret(secret),
            lookup_tables: Arc::new(lookup_tables),
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

impl FromRef<AppState> for Arc<DatabaseConnection> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
