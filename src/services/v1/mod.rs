pub mod auth;
pub mod core;

use std::sync::Arc;
use std::collections::HashMap;
use jsonwebtoken::EncodingKey;

use sea_orm::DatabaseConnection;

use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::mq::RabbitMQClient;
use crate::core::state::{AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
pub use auth::AuthService;

/// Factory function to create AuthService with its dependencies.
/// This acts as the injection point for the service's infrastructure needs.
pub fn init_auth_service(
    db: DatabaseConnection,
    valkey: Arc<ValkeyClient>,
    rabbitmq: Arc<RabbitMQClient>,
    templates: Arc<HashMap<String, String>>,
    access_token_ttl: AccessTokenDefaultTTLSeconds,
    session_ttl: SessionDefaultTTLSeconds,
    encoding_key: EncodingKey,
) -> AuthService {
    AuthService::new(
        db,
        valkey,
        rabbitmq,
        templates,
        access_token_ttl,
        session_ttl,
        encoding_key,
    )
}
