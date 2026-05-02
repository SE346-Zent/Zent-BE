use axum::extract::FromRef;
use std::sync::Arc;
use std::collections::HashMap;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;
use lapin::Connection;

use crate::core::lookup_tables::LookupTables;
use crate::services::v1::auth::AuthService;
use crate::infrastructure::cache::ValkeyClient;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

/// AppState now holds both infrastructure resources (db, cache, mq)
/// and the service registry. This enables Handlers to act as Orchestrators
/// by accessing infrastructure directly while calling pure logic from services.
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
    pub auth_service: Arc<AuthService>,
}

impl AppState {
    pub fn new(
        secret: &[u8],
        lookup_tables: LookupTables,
        auth_service: AuthService,
    ) -> Self {
        // Extract infrastructure from auth_service to maintain compatibility with existing tests
        // that instantiate AppState via this constructor.
        let db = Arc::new(auth_service.get_db().clone());
        let valkey = auth_service.get_valkey().clone();
        let rabbitmq = auth_service.get_rabbitmq().clone();
        let templates = auth_service.get_templates().clone();
        let access_token_ttl = auth_service.get_access_token_ttl();
        let session_ttl = auth_service.get_session_ttl();

        Self {
            db,
            valkey,
            rabbitmq,
            templates,
            access_token_ttl,
            session_ttl,
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

impl FromRef<AppState> for Arc<DatabaseConnection> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
