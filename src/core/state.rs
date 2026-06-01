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

/// Shared application state containing all infrastructure resources and global configuration.
#[derive(Clone)]
pub struct AppState {
    /// MySQL database connection pool (via SeaORM).
    pub db: Arc<DatabaseConnection>,
    /// MongoDB database connection for document-oriented data.
    pub mongodb: Arc<MongoDatabase>,
    /// Valkey/Redis client for caching and session management.
    pub valkey: Option<Arc<ValkeyClient>>,
    /// RabbitMQ connection for asynchronous message processing.
    pub rabbitmq: Option<Arc<Connection>>,
    /// Pre-loaded HTML email templates.
    pub templates: Arc<HashMap<String, String>>,
    /// Default Time-To-Live for access tokens.
    pub access_token_ttl: AccessTokenDefaultTTLSeconds,
    /// Default Time-To-Live for user sessions.
    pub session_ttl: SessionDefaultTTLSeconds,
    /// Key used for decoding/verifying JWT tokens.
    pub decoding_key: DecodingKey,
    /// Key used for encoding/signing JWT tokens.
    pub encoding_key: EncodingKey,
    /// In-memory cache of frequently used lookup tables.
    pub lookup_tables: Arc<LookupTables>,
    /// Client for third-party Zeus API
    pub zeus_client: Arc<dyn crate::services::v1::inventory::ports::ZeusInventoryClient>,
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
        zeus_client: Arc<dyn crate::services::v1::inventory::ports::ZeusInventoryClient>,
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
            zeus_client,
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
