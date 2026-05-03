use axum::extract::FromRef;
use std::sync::Arc;
use std::collections::HashMap;
use jsonwebtoken::{DecodingKey, EncodingKey};
use sea_orm::DatabaseConnection;
use lapin::Connection;

use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;
use crate::services::v1::auth::AuthService;
use crate::services::v1::work_orders::WorkOrderService;
use crate::services::v1::core::media::MediaService;

#[derive(Clone, Copy)]
pub struct AccessTokenDefaultTTLSeconds(pub i64);

#[derive(Clone, Copy)]
pub struct SessionDefaultTTLSeconds(pub i64);

/// AppState holds infrastructure resources (db, cache, mq) directly.
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
    pub auth_service: Arc<AuthService>,
    pub work_order_service: Arc<WorkOrderService>,
    pub media_service: Arc<MediaService>,
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
        auth_service: AuthService,
        work_order_service: WorkOrderService,
        media_service: MediaService,
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
            auth_service: Arc::new(auth_service),
            work_order_service: Arc::new(work_order_service),
            media_service: Arc::new(media_service),
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

impl FromRef<AppState> for Arc<AuthService> {
    fn from_ref(state: &AppState) -> Self {
        state.auth_service.clone()
    }
}

impl FromRef<AppState> for Arc<WorkOrderService> {
    fn from_ref(state: &AppState) -> Self {
        state.work_order_service.clone()
    }
}

impl FromRef<AppState> for Arc<MediaService> {
    fn from_ref(state: &AppState) -> Self {
        state.media_service.clone()
    }
}
