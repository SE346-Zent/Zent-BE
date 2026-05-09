use std::collections::HashMap;
use sea_orm::{DatabaseConnection, Set, ActiveModelTrait};
use zent_be::entities::{roles, account_status, work_order_statuses, work_order_symptoms, users, policy};
use zent_be::core::state::{AppState, AccessTokenDefaultTTLSeconds, SessionDefaultTTLSeconds};
use zent_be::core::lookup_tables::LookupTables;
use zent_be::model::jwt_claims::Claims;
use jsonwebtoken::{EncodingKey, Header};
use uuid::Uuid;

pub const WO_STATUSES: &[&str] = &["Pending", "Assigned", "In Progress", "Closed", "Reject_InReview", "Rejected"];
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

// Hard-coded test user UUIDs for deterministic JWT generation
pub const TEST_ADMIN_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
pub const TEST_TECHNICIAN_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
pub const TEST_CUSTOMER_ID: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";

// A fixed secret for test JWT signing
pub const TEST_JWT_SECRET: &[u8] = b"test_secret_key_for_jwt_signing_32b";

// ---------------------------------------------------------
// Boundary Initialization
// ---------------------------------------------------------

pub async fn seed_test_db(db: &DatabaseConnection) {
    let now = chrono::Utc::now();

    // Roles — matching production seeder (seeder/src/role.rs): Admin, SuperAdmin, Customer, Technician
    let _ = roles::ActiveModel { id: Set(1), name: Set("Admin".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(2), name: Set("SuperAdmin".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(3), name: Set("Customer".to_string()) }.insert(db).await;
    let _ = roles::ActiveModel { id: Set(4), name: Set("Technician".to_string()) }.insert(db).await;

    // Account statuses
    let _ = account_status::ActiveModel { id: Set(1), name: Set("Pending".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(2), name: Set("Active".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(3), name: Set("Inactive".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(4), name: Set("Locked".to_string()) }.insert(db).await;
    let _ = account_status::ActiveModel { id: Set(5), name: Set("Terminated".to_string()) }.insert(db).await;

    // Work order statuses
    for (i, &name) in WO_STATUSES.iter().enumerate() {
        let _ = work_order_statuses::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), ..Default::default() }.insert(db).await;
    }

    // Work order symptoms
    for (i, &name) in WORK_ORDER_SYMPTOMS.iter().enumerate() {
        let _ = work_order_symptoms::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), created_at: Set(now), updated_at: Set(now), ..Default::default() }.insert(db).await;
    }

    // Policies (geofencing_radius, workday_start, workday_end)
    let _ = policy::ActiveModel { id: Set(1), policy_name: Set("geofencing_radius".to_string()), policy_value: Set("5000000.0".to_string()) }.insert(db).await;
    let _ = policy::ActiveModel { id: Set(2), policy_name: Set("workday_start".to_string()), policy_value: Set("8".to_string()) }.insert(db).await;
    let _ = policy::ActiveModel { id: Set(3), policy_name: Set("workday_end".to_string()), policy_value: Set("18".to_string()) }.insert(db).await;

    // Test users
    let admin_id: Uuid = Uuid::parse_str(TEST_ADMIN_ID).unwrap();
    let technician_id: Uuid = Uuid::parse_str(TEST_TECHNICIAN_ID).unwrap();
    let customer_id: Uuid = Uuid::parse_str(TEST_CUSTOMER_ID).unwrap();

    let _ = users::ActiveModel {
        id: Set(admin_id),
        account_status: Set(2), // Active
        role_id: Set(1),        // Admin
        email: Set("admin@test.com".to_string()),
        full_name: Set("Test Admin".to_string()),
        password_hash: Set("hash".to_string()),
        phone_number: Set("+84111111111".to_string()),
        province: Set(Some("HCM".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await;

    let _ = users::ActiveModel {
        id: Set(technician_id),
        account_status: Set(2), // Active
        role_id: Set(4),        // Technician
        email: Set("tech@test.com".to_string()),
        full_name: Set("Test Technician".to_string()),
        password_hash: Set("hash".to_string()),
        phone_number: Set("+84222222222".to_string()),
        province: Set(Some("HCM".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await;

    let _ = users::ActiveModel {
        id: Set(customer_id),
        account_status: Set(2), // Active
        role_id: Set(3),        // Customer
        email: Set("cust@test.com".to_string()),
        full_name: Set("Test Customer".to_string()),
        password_hash: Set("hash".to_string()),
        phone_number: Set("+84333333333".to_string()),
        province: Set(Some("HCM".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await;
}

pub async fn seed_work_order(
    db: &DatabaseConnection,
    id: Uuid,
    technician_id: Option<Uuid>,
    status_id: i32,
    reject_form_id: Option<Uuid>,
) -> zent_be::entities::work_orders::Model {
    let now = chrono::Utc::now();

    // 1. Seed Product Model if not exists
    let model_code = "TEST_MODEL";
    let _ = zent_be::entities::product_models::ActiveModel {
        model_code: Set(model_code.to_string()),
        model_name: Set("Test Model".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await;

    // 2. Seed Product
    let product_id = Uuid::new_v4();
    let customer_id = Uuid::parse_str(TEST_CUSTOMER_ID).unwrap();
    let _ = zent_be::entities::products::ActiveModel {
        id: Set(product_id),
        product_model_code: Set(model_code.to_string()),
        customer_id: Set(customer_id),
        product_name: Set("Test Product".to_string()),
        serial_number: Set("SN123456".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await;

    // 3. Seed Work Order
    zent_be::entities::work_orders::ActiveModel {
        id: Set(id),
        work_order_status_id: Set(status_id),
        customer_id: Set(customer_id),
        product_id: Set(product_id),
        work_order_symptom_id: Set(1),
        description: Set("Test Description".to_string()),
        first_name: Set("Test".to_string()),
        last_name: Set("User".to_string()),
        country: Set("Vietnam".to_string()),
        province: Set("Ho Chi Minh City".to_string()),
        city: Set("Ho Chi Minh City".to_string()),
        address: Set("Ho Chi Minh City".to_string()),
        appointment: Set(now),
        work_order_number: Set(format!("WO-{}", id.to_string()[..8].to_uppercase())),
        technician_id: Set(technician_id),
        reject_form_id: Set(reject_form_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to seed work order")
}

pub async fn seed_image_link(
    db: &DatabaseConnection,
    work_order_id: Uuid,
    phase: &str,
) -> Uuid {
    let now = chrono::Utc::now();
    let image_id = Uuid::new_v4();

    // 1. Seed Image
    let _ = zent_be::entities::images::ActiveModel {
        id: Set(image_id),
        object_name: Set(format!("test_image_{}.jpg", image_id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to seed image");

    // 2. Seed Link
    let _ = zent_be::entities::work_order_image_links::ActiveModel {
        image_id: Set(image_id),
        work_order_id: Set(work_order_id),
        phase: Set(phase.to_string()),
        latitude: Set(Some(10.762622)),
        longitude: Set(Some(106.660172)),
        is_verified: Set(true),
    }
    .insert(db)
    .await
    .expect("Failed to seed image link");

    image_id
}

pub async fn seed_reject_form(
    db: &DatabaseConnection,
    id: Uuid,
) -> zent_be::entities::work_order_reject_forms::Model {
    zent_be::entities::work_order_reject_forms::ActiveModel {
        id: Set(id),
        reason: Set("Test Reason".to_string()),
        explanation: Set("Test Explanation".to_string()),
        approved: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("Failed to seed reject form")
}

// ---------------------------------------------------------
// Test AppState factory
// ---------------------------------------------------------

pub async fn create_test_app_state(db: DatabaseConnection) -> AppState {
    let lookup_tables = LookupTables::load(&db).await.expect("Failed to load lookup tables");

    AppState::new(
        TEST_JWT_SECRET,
        lookup_tables,
        db,
        None,   // valkey
        None,   // rabbitmq
        HashMap::new(), // templates
        AccessTokenDefaultTTLSeconds(3600),
        SessionDefaultTTLSeconds(86400),
    )
}

// ---------------------------------------------------------
// JWT helper — generate a valid token for a test user
// ---------------------------------------------------------

pub fn create_test_jwt(user_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + 3600,
    };
    let key = EncodingKey::from_secret(TEST_JWT_SECRET);
    jsonwebtoken::encode(&Header::default(), &claims, &key).expect("Failed to encode test JWT")
}
