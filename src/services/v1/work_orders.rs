use sea_orm::DatabaseConnection;
use std::sync::Arc;
use lapin::Connection;
use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;

#[derive(Clone)]
pub struct WorkOrderService {
    db: DatabaseConnection,
    luts: Arc<LookupTables>,
    valkey: Option<Arc<ValkeyClient>>,
    rabbitmq: Option<Arc<Connection>>,
}

impl WorkOrderService {
    pub fn new(
        db: DatabaseConnection,
        luts: Arc<LookupTables>,
        valkey: Option<Arc<ValkeyClient>>,
        rabbitmq: Option<Arc<Connection>>,
    ) -> Self {
        Self {
            db,
            luts,
            valkey,
            rabbitmq,
        }
    }

    pub async fn create(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn list(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn get_details(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn assign(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn schedule(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn start(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn refuse(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn cancel(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn complete(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn history(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn add_parts(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn approve_refusal(&self) -> Result<(), ()> { unimplemented!() }
    pub async fn deny_refusal(&self) -> Result<(), ()> { unimplemented!() }
}
