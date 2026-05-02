use std::sync::Arc;
use sea_orm::{DatabaseConnection, Set, ActiveModelTrait};
use zent_be::entities::{roles, account_status, work_order_statuses, work_order_symptoms};
use zent_be::services::v1::work_orders::WorkOrderService;
use zent_be::services::v1::core::media::MediaService;
use axum::extract::FromRef;

pub const WO_STATUSES: &[&str] = &["Pending", "Assigned", "InProg", "Closed", "Reject_InReview", "Rejected"];
pub const WORK_ORDER_SYMPTOMS: &[&str] = &[
    "Active Noise Cancelling(ANC)", "Backpack", "Bluetooth", "Case", "Charger", "External Hot Spot Issue",
    "External Keyboard", "External Mouse", "External Storage(USB/SSD/etc)", "Glasses", "Headset",
    "Kit(Mouse and Keyboard)", "MousePad", "Other", "PC Port not working properly", "Pen", "Printer",
    "Web Camera", "Audio", "Battery", "Boot issue", "Branding", "Camera", "Charging", "Covers", "Display",
    "Dock", "Drive (SSD / HDD)", "External Display", "Fan", "Fingerprint", "Keyboards", "Network",
    "No Post", "No Power", "Noise", "Non Technical", "Operating System (OS)", "Performance",
    "Physical Damage (CID)", "Physical Damage (Not CID)", "Pointing Devices", "Power Button",
    "Safety issue", "Smart card reader", "Smart Collab", "Software", "USB Port", "Other"
];

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

    for (i, &name) in WO_STATUSES.iter().enumerate() {
        let _ = work_order_statuses::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), ..Default::default() }.insert(db).await;
    }

    let now = chrono::Utc::now();
    for (i, &name) in WORK_ORDER_SYMPTOMS.iter().enumerate() {
        let _ = work_order_symptoms::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), created_at: Set(now), updated_at: Set(now), ..Default::default() }.insert(db).await;
    }
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
