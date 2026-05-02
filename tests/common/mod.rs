use std::sync::Arc;
use sea_orm::{DatabaseConnection, Set, ActiveModelTrait};
use zent_be::entities::{roles, account_status};
use zent_be::services::v1::work_orders::WorkOrderService;
use zent_be::services::v1::media::MediaService;
use axum::extract::FromRef;

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

pub async fn seed_test_db(db: &DatabaseConnection) {
    let _ = roles::ActiveModel { id: Set(1), name: Set("Admin".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(2), name: Set("Manager".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(3), name: Set("Technician".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(4), name: Set("Dispatcher".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(5), name: Set("Customer".to_string()) }.insert(db).await;

    let _ = account_status::ActiveModel { id: Set(1), name: Set("Pending".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(2), name: Set("Active".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(3), name: Set("Inactive".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(4), name: Set("Locked".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(5), name: Set("Terminated".to_string()) }.insert(db).await;
}

// ---------------------------------------------------------
// Test State
// ---------------------------------------------------------
#[derive(Clone)]
pub struct WorkOrderTestState {
    pub work_order_service: Arc<WorkOrderService>,
    pub media_service: Arc<MediaService>,
}

impl FromRef<WorkOrderTestState> for Arc<WorkOrderService> {
    fn from_ref(state: &WorkOrderTestState) -> Self {
        state.work_order_service.clone()
    }
}

impl FromRef<WorkOrderTestState> for Arc<MediaService> {
    fn from_ref(state: &WorkOrderTestState) -> Self {
        state.media_service.clone()
    }
}
