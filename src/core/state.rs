use axum::extract::FromRef;
use std::sync::Arc;
use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::core::lookup_tables::LookupTables;
use crate::services::v1::auth::AuthService;
use crate::services::v1::work_orders::WorkOrderService;
use crate::services::v1::media::MediaService;

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
    pub work_order_service: Arc<WorkOrderService>,
    pub media_service: Arc<MediaService>,
}

impl AppState {
    /// AppState now strictly acts as a ServiceRegistry.
    /// It only requires the JWT secret, lookup tables, and service instances.
    pub fn new(
        secret: &[u8],
        lookup_tables: LookupTables,
        auth_service: AuthService,
        work_order_service: WorkOrderService,
        media_service: MediaService,
    ) -> Self {
        Self {
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
