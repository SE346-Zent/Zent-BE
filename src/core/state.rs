use axum::extract::FromRef;
use std::sync::Arc;
use std::collections::HashMap;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;
use lapin::Connection;
use mongodb::Database as MongoDatabase;

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

/// AppState holds infrastructure resources (db, cache, mq) directly.
/// This enables Handlers to act as Actors by extracting infrastructure
/// directly using FromRef and executing side-effects.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub mongodb: Arc<MongoDatabase>,
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
        mongodb: MongoDatabase,
        valkey: Option<Arc<ValkeyClient>>,
        rabbitmq: Option<Arc<Connection>>,
        templates: HashMap<String, String>,
        access_token_ttl: AccessTokenDefaultTTLSeconds,
        session_ttl: SessionDefaultTTLSeconds,
    ) -> Self {
        Self {
            db: Arc::new(db),
            mongodb: Arc::new(mongodb),
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

impl FromRef<AppState> for Option<Arc<ValkeyClient>> {
    fn from_ref(state: &AppState) -> Self {
        state.valkey.clone()
    }
}

impl FromRef<AppState> for Option<Arc<Connection>> {
    fn from_ref(state: &AppState) -> Self {
        state.rabbitmq.clone()
    }
}

impl FromRef<AppState> for Arc<HashMap<String, String>> {
    fn from_ref(state: &AppState) -> Self {
        state.templates.clone()
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

impl FromRef<AppState> for Arc<MongoDatabase> {
    fn from_ref(state: &AppState) -> Self {
        state.mongodb.clone()
    }
}
