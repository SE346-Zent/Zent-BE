use sea_orm::DatabaseConnection;
use std::sync::Arc;
use lapin::Connection;
use crate::core::lookup_tables::LookupTables;
use crate::infrastructure::cache::ValkeyClient;

mod create;
mod list;
mod get_details;
mod assign;
mod schedule;
mod start;
mod refuse;
mod cancel;
mod complete;
mod history;
mod add_parts;
mod approve_refusal;
mod deny_refusal;

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

    pub async fn create(&self) -> Result<(), ()> {
        create::handle_create(&self.db).await
    }

    pub async fn list(&self) -> Result<(), ()> {
        list::handle_list(&self.db).await
    }

    pub async fn get_details(&self) -> Result<(), ()> {
        get_details::handle_get_details(&self.db).await
    }

    pub async fn assign(&self) -> Result<(), ()> {
        assign::handle_assign(&self.db).await
    }

    pub async fn schedule(&self) -> Result<(), ()> {
        schedule::handle_schedule(&self.db).await
    }

    pub async fn start(&self) -> Result<(), ()> {
        start::handle_start(&self.db).await
    }

    pub async fn refuse(&self) -> Result<(), ()> {
        refuse::handle_refuse(&self.db).await
    }

    pub async fn cancel(&self) -> Result<(), ()> {
        cancel::handle_cancel(&self.db).await
    }

    pub async fn complete(&self) -> Result<(), ()> {
        complete::handle_complete(&self.db).await
    }

    pub async fn history(&self) -> Result<(), ()> {
        history::handle_history(&self.db).await
    }

    pub async fn add_parts(&self) -> Result<(), ()> {
        add_parts::handle_add_parts(&self.db).await
    }

    pub async fn approve_refusal(&self) -> Result<(), ()> {
        approve_refusal::handle_approve_refusal(&self.db).await
    }

    pub async fn deny_refusal(&self) -> Result<(), ()> {
        deny_refusal::handle_deny_refusal(&self.db).await
    }
}
