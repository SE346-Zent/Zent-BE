use sea_orm::DatabaseConnection;
use std::sync::Arc;
use lapin::Connection;
use crate::infrastructure::cache::ValkeyClient;

#[derive(Clone)]
pub struct MediaService {
    db: DatabaseConnection,
    valkey: Option<Arc<ValkeyClient>>,
    rabbitmq: Option<Arc<Connection>>,
}

impl MediaService {
    pub fn new(
        db: DatabaseConnection,
        valkey: Option<Arc<ValkeyClient>>,
        rabbitmq: Option<Arc<Connection>>,
    ) -> Self {
        Self {
            db,
            valkey,
            rabbitmq,
        }
    }

    pub async fn upload_media(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn get_media(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn list_media(&self) -> Result<(), ()> { unimplemented!() }
}
