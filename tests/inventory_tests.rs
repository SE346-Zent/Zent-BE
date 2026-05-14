//! Integration tests for the inventory domain.
//!
//! Tests the service layer directly with an in-memory SQLite database.
//! Since handlers are stubbed with `unimplemented!()`, these tests
//! exercise the pure service logic by calling service functions directly,
//! seeded with realistic data.

use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection, EntityTrait, ActiveModelTrait, Set, QueryFilter, ColumnTrait};
use uuid::Uuid;

use zent_be::core::errors::AppError;
use zent_be::services::v1::inventory::parts::{self, RawPart};
use zent_be::services::v1::inventory::products::{self, RawProduct};
use zent_be::services::v1::inventory::approve_part;
use zent_be::model::requests::inventory::list_parts_query::ListPartsQuery;
use zent_be::model::requests::inventory::list_products_query::ListProductsQuery;
use zent_be::model::requests::inventory::register_product_request::RegisterProductRequest;
use zent_be::entities::{roles, account_status, work_order_statuses, work_order_symptoms};

fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }

// ─────────────────────────────────────────────────────────────────────────────
// Database helper factory
// ─────────────────────────────────────────────────────────────────────────────

async fn seeded_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    Migrator::up(&db, None).await.unwrap();

    // Insert roles
    let _ = roles::ActiveModel { id: Set(1), name: Set("Admin".to_string()) }.insert(&db).await;
    let _ = roles::ActiveModel { id: Set(3), name: Set("Technician".to_string()) }.insert(&db).await;
    let _ = roles::ActiveModel { id: Set(5), name: Set("Customer".to_string()) }.insert(&db).await;

    // Insert account statuses
    let _ = account_status::ActiveModel { id: Set(2), name: Set("Active".to_string()) }.insert(&db).await;

    // Work order statuses (needed for FK in work_orders if referenced)
    for (i, name) in ["Pending", "Assigned", "InProg", "Closed", "Reject_InReview", "Rejected"].iter().enumerate() {
        let _ = zent_be::entities::work_order_statuses::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), ..Default::default() }.insert(&db).await;
    }

    // Work order symptoms
    let now = chrono::Utc::now();
    for (i, name) in ["Battery", "Display", "Charger", "Audio"].iter().enumerate() {
        let _ = zent_be::entities::work_order_symptoms::ActiveModel { id: Set(i as i32 + 1), name: Set(name.to_string()), created_at: Set(now), updated_at: Set(now), ..Default::default() }.insert(&db).await;
    }

    db
}

/// Create a RawPart for testing the service layer.
fn make_raw_part(
    part_id: &str, part_number: &str, serial: &str, approval: &str,
    type_name: &str, condition: &str, tech_id: Option<&str>, cust_id: Option<&str>,
    product_id: Option<&str>, product_name: Option<&str>, created: &str,
) -> RawPart {
    RawPart {
        part_id: u(part_id), part_number: part_number.to_string(),
        part_type_id: 1, part_type_name: type_name.to_string(),
        model_code: Some("MODEL-A".to_string()), serial_number: serial.to_string(),
        description: None, condition_id: 1, condition_name: condition.to_string(),
        product_id: product_id.map(|s| u(s)), product_name: product_name.map(|s| s.to_string()),
        work_order_id: Some(u("00000000-0000-0000-0000-000000000001")),
        technician_id: tech_id.map(|s| u(s)), customer_id: cust_id.map(|s| u(s)),
        approval_status: approval.to_string(), denial_reason: None,
        manufactured_date: None, installation_date: None,
        created_at: created.to_string(), updated_at: "2024-06-01T00:00:00Z".to_string(),
    }
}

fn make_raw_product(
    id: &str, name: &str, model: &str, serial: &str, cust_id: &str,
    cust_name: &str, parts_tech_ids: Vec<&str>, created: &str,
) -> RawProduct {
    RawProduct {
        product_id: u(id), product_name: name.to_string(), model_code: model.to_string(),
        model_name: format!("Model {}", model), serial_number: serial.to_string(),
        customer_id: u(cust_id), customer_name: cust_name.to_string(),
        part_count: parts_tech_ids.len() as i64,
        parts: parts_tech_ids.iter().enumerate().map(|(i, tid)| {
            make_raw_part(
                &format!("{i}000000-0000-0000-0000-000000000000"), &format!("PN-{i}"),
                &format!("SN-{i}"), "approved", "Battery", "New",
                Some(tid), Some(cust_id), Some(id), Some(name), "2024-01-01T00:00:00Z",
            )
        }).collect(),
        created_at: created.to_string(), updated_at: "2024-06-01T00:00:00Z".to_string(),
    }
}

fn default_parts_query() -> ListPartsQuery {
    ListPartsQuery { model_code: None, part_type_id: None, approval_status: None, search: None, page: None, limit: None }
}

fn default_products_query() -> ListProductsQuery {
    ListProductsQuery { model_code: None, search: None, page: None, limit: None }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 1 — Complex role-access matrix across 5 users and 10 parts
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_role_access_matrix_10_parts_5_users() {
    let _db = seeded_db().await;
    let admin   = u("a0000000-0000-0000-0000-000000000000");
    let tech_a  = u("b0000000-0000-0000-0000-000000000001");
    let tech_b  = u("b0000000-0000-0000-0000-000000000002");
    let cust_a  = u("c0000000-0000-0000-0000-000000000001");
    let cust_b  = u("c0000000-0000-0000-0000-000000000002");

    let parts = vec![
        // Tech A, Customer A product, pending
        make_raw_part("10000000-0000-0000-0000-000000000000", "BAT-001", "S-001", "pending", "Battery", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-05-01T00:00:00Z"),
        // Tech A, Customer A product, approved
        make_raw_part("20000000-0000-0000-0000-000000000000", "SCR-001", "S-002", "approved", "Screen", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-05-02T00:00:00Z"),
        // Tech A, Customer B product, approved
        make_raw_part("30000000-0000-0000-0000-000000000000", "CHG-001", "S-003", "approved", "Charger", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000002"), Some("p0000000-0000-0000-0000-000000000002"), Some("Laptop B"), "2024-05-03T00:00:00Z"),
        // Tech B, Customer B product, approved
        make_raw_part("40000000-0000-0000-0000-000000000000", "BAT-002", "S-004", "approved", "Battery", "Used", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000002"), Some("p0000000-0000-0000-0000-000000000002"), Some("Laptop B"), "2024-05-04T00:00:00Z"),
        // Tech B, no customer, pending (not assigned to product yet)
        make_raw_part("50000000-0000-0000-0000-000000000000", "AUD-001", "S-005", "pending", "Audio", "New", Some("b0000000-0000-0000-0000-000000000002"), None, None, None, "2024-05-05T00:00:00Z"),
        // No tech (orphan), approved, Customer A
        make_raw_part("60000000-0000-0000-0000-000000000000", "DSP-001", "S-006", "approved", "Display", "New", None, Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-05-06T00:00:00Z"),
        // Tech A, denied, Customer A
        make_raw_part("70000000-0000-0000-0000-000000000000", "FAN-001", "S-007", "denied", "Fan", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-05-07T00:00:00Z"),
        // Tech B, Approved, Customer A (cross: different tech, same customer)
        make_raw_part("80000000-0000-0000-0000-000000000000", "KBD-001", "S-008", "approved", "Keyboard", "New", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-05-08T00:00:00Z"),
        // Tech A, approved, Customer A (different product)
        make_raw_part("90000000-0000-0000-0000-000000000000", "MOU-001", "S-009", "approved", "Mouse", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), Some("p0000000-0000-0000-0000-000000000003"), Some("Desktop A"), "2024-05-09T00:00:00Z"),
        // Tech A, approved, no customer, no product (approved orphan)
        make_raw_part("a0000000-0000-0000-0000-000000000000", "PWR-001", "S-010", "approved", "Power Supply", "New", Some("b0000000-0000-0000-0000-000000000001"), None, None, None, "2024-05-10T00:00:00Z"),
    ];

    // Admin: sees all 10
    let (admin_items, admin_meta) = parts::list_parts(&parts, "Admin", admin, &default_parts_query());
    assert_eq!(admin_items.len(), 10, "Admin should see all 10 parts");
    assert_eq!(admin_meta.total_records, 10);

    // Tech A: sees parts where they are technician (6 parts: indices 0,1,2,6,8,9)
    let (ta_items, _) = parts::list_parts(&parts, "Technician", tech_a, &default_parts_query());
    assert_eq!(ta_items.len(), 6, "Tech A should see 6 parts assigned to them");
    let ta_part_nos: Vec<&str> = ta_items.iter().map(|p| p.part_number.as_str()).collect();
    assert!(ta_part_nos.contains(&"BAT-001"), "Tech A should see BAT-001");
    assert!(ta_part_nos.contains(&"FAN-001"), "Tech A should see FAN-001 even if denied");
    assert!(!ta_part_nos.contains(&"BAT-002"), "Tech A should NOT see Tech B's BAT-002");

    // Tech B: sees parts where they are technician (3 parts: indices 3,4,7)
    let (tb_items, _) = parts::list_parts(&parts, "Technician", tech_b, &default_parts_query());
    assert_eq!(tb_items.len(), 3, "Tech B should see 3 parts");
    let tb_part_nos: Vec<&str> = tb_items.iter().map(|p| p.part_number.as_str()).collect();
    assert!(tb_part_nos.contains(&"BAT-002"), "Tech B should see BAT-002");
    assert!(tb_part_nos.contains(&"AUD-001"), "Tech B should see AUD-001 (pending, no product)");
    assert!(!tb_part_nos.contains(&"BAT-001"), "Tech B should NOT see Tech A's parts");

    // Customer A: sees approved parts in their products only (3 items: S-002, S-006, S-008, S-009)
    // But S-001 is pending → not visible; S-007 is denied → not visible
    let (ca_items, _) = parts::list_parts(&parts, "Customer", cust_a, &default_parts_query());
    assert_eq!(ca_items.len(), 4, "Customer A should see 4 approved parts in their products");
    let ca_serials: Vec<&str> = ca_items.iter().map(|p| p.serial_number.as_str()).collect();
    assert!(ca_serials.contains(&"S-002"));
    assert!(ca_serials.contains(&"S-006"));
    assert!(ca_serials.contains(&"S-008"));
    assert!(ca_serials.contains(&"S-009"));
    assert!(!ca_serials.contains(&"S-001"), "Customer should NOT see pending part S-001");
    assert!(!ca_serials.contains(&"S-007"), "Customer should NOT see denied part S-007");
    assert!(!ca_serials.contains(&"S-004"), "Customer should NOT see other customer's part S-004");

    // Customer B: sees approved parts in their products only (2 items: S-003, S-004)
    let (cb_items, _) = parts::list_parts(&parts, "Customer", cust_b, &default_parts_query());
    assert_eq!(cb_items.len(), 2, "Customer B should see 2 approved parts");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 2 — Part approval/denial state machine with full audit trail
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_approval_state_machine_with_audit() {
    let _db = seeded_db().await;
    let admin_a = u("a0000000-0000-0000-0000-000000000001");
    let admin_b = u("a0000000-0000-0000-0000-000000000002");
    let form_id = u("f0000000-0000-0000-0000-000000000001");
    let now = chrono::Utc::now();

    // 1. Accept a pending form → succeeds
    let accept = approve_part::decide_accept_part(form_id, admin_a, "pending", now);
    assert!(accept.is_ok(), "Should accept pending form");
    let effect = accept.unwrap();
    assert_eq!(effect.audit.action, "approved");
    assert!(effect.audit.reason.is_none());
    assert_eq!(effect.audit.admin_id, admin_a);
    // Part ID should be freshly generated
    assert_ne!(effect.part_id, uuid::Uuid::nil());

    // 2. Accept an already-approved form → rejected
    let double_accept = approve_part::decide_accept_part(form_id, admin_b, "approved", now);
    assert!(double_accept.is_err());

    // 3. Deny an already-approved form → rejected (can't deny approved)
    let deny_approved = approve_part::decide_deny_part(
        form_id, admin_a, "approved",
        "This part was already approved and cannot be denied",
        now,
    );
    assert!(deny_approved.is_err());

    // 4. Deny a pending form → succeeds
    let form2 = u("f0000000-0000-0000-0000-000000000002");
    let deny = approve_part::decide_deny_part(
        form2, admin_b, "pending",
        "Part does not meet quality requirements; incorrect dimensions",
        now,
    );
    assert!(deny.is_ok());
    let deny_effect = deny.unwrap();
    assert_eq!(deny_effect.audit.action, "denied");
    assert_eq!(deny_effect.audit.reason.unwrap(), "Part does not meet quality requirements; incorrect dimensions");
    assert_eq!(deny_effect.audit.admin_id, admin_b);

    // 5. Deny an already-denied form → rejected (can't deny twice)
    let double_deny = approve_part::decide_deny_part(
        form2, admin_a, "denied",
        "Attempting to deny an already denied part",
        now,
    );
    assert!(double_deny.is_err());

    // 6. Deny with very short reason → rejected
    let short_deny = approve_part::decide_deny_part(
        u("f0000000-0000-0000-0000-000000000003"), admin_a, "pending",
        "short", // 5 chars
        now,
    );
    assert!(short_deny.is_err());

    // 7. Deny with just barely valid reason (10 chars) → valid
    let exact_deny = approve_part::decide_deny_part(
        u("f0000000-0000-0000-0000-000000000004"), admin_b, "pending",
        "0123456789", // exactly 10 chars
        now,
    );
    assert!(exact_deny.is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 3 — Product registration with duplicate detection, idempotency, catalog validation
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_product_registration_complex() {
    let _db = seeded_db().await;
    let user_a = u("c0000000-0000-0000-0000-000000000001");
    let user_b = u("c0000000-0000-0000-0000-000000000002");
    let now = chrono::Utc::now();

    // Make a valid registration request
    let req = RegisterProductRequest {
        serial_number: "SN-VALID-001".to_string(),
        country: "Vietnam".to_string(),
        province: "Ho Chi Minh".to_string(),
        city: "District 1".to_string(),
        address: "456 Nguyen Hue".to_string(),
        first_name: "Minh".to_string(),
        last_name: "Tran".to_string(),
        email: "minh@example.com".to_string(),
        mobile_phone: "0987654321".to_string(),
        send_email_confirmation: true,
    };

    // Initial registration → success
    let r1 = products::decide_register_product(
        &req, user_a, "Minh Tran",
        Some("MODEL-X".to_string()), Some("ThinkPad X1 Carbon".to_string()),
        None, now,
    );
    assert!(r1.is_ok(), "First registration should succeed");
    let e1 = r1.unwrap();
    assert!(e1.should_send_email, "Email should be sent");
    assert_eq!(e1.serial_number, "SN-VALID-001");

    // Second registration by same user with same serial → idempotent (returns existing product)
    let existing_product = make_raw_product(
        "p0000000-0000-0000-0000-000000000001", "ThinkPad X1 Carbon Vietnam 2026",
        "MODEL-X", "SN-VALID-001", "c0000000-0000-0000-0000-000000000001",
        "Minh Tran", vec![], "2026-05-01T00:00:00Z",
    );
    let r2 = products::decide_register_product(
        &req, user_a, "Minh Tran",
        Some("MODEL-X".to_string()), Some("ThinkPad X1 Carbon".to_string()),
        Some(&existing_product), now,
    );
    assert!(r2.is_ok(), "Re-registration by same user should be idempotent");
    let e2 = r2.unwrap();
    assert_eq!(e2.product_id, existing_product.product_id, "Should return existing product ID");
    assert!(!e2.should_send_email, "Email should NOT be resent");

    // Third: different user tries same serial → Conflict
    let r3 = products::decide_register_product(
        &req, user_b, "Lan Pham",
        Some("MODEL-X".to_string()), Some("ThinkPad X1 Carbon".to_string()),
        Some(&existing_product), now,
    );
    assert!(r3.is_err(), "Different user should get Conflict");
    match r3.unwrap_err() {
        AppError::Conflict(msg) => assert!(msg.contains("already registered")),
        other => panic!("Expected Conflict, got {:?}", other),
    }

    // Fourth: serial not in catalog → BadRequest
    let r4 = products::decide_register_product(
        &req, user_a, "Minh Tran",
        None, // Serial not found
        None,
        None, now,
    );
    assert!(r4.is_err());

    // Fifth: empty country defaults to Vietnam in product name
    let mut req_no_country = req.clone();
    req_no_country.country = "".to_string();
    let r5 = products::decide_register_product(
        &req_no_country, user_a, "Minh Tran",
        Some("MODEL-X".to_string()), Some("ThinkPad X1 Carbon".to_string()),
        None, now,
    );
    let e5 = r5.unwrap();
    assert!(e5.product_name.contains("Vietnam"), "Product name should default to Vietnam");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 4 — Product listing with cross-tech, cross-customer isolation
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_product_isolation_across_roles() {
    let _db = seeded_db().await;
    let tech_a = u("b0000000-0000-0000-0000-000000000001");
    let tech_b = u("b0000000-0000-0000-0000-000000000002");
    let tech_c = u("b0000000-0000-0000-0000-000000000003");
    let cust_a = u("c0000000-0000-0000-0000-000000000001");
    let cust_b = u("c0000000-0000-0000-0000-000000000002");
    let admin = u("a0000000-0000-0000-0000-000000000000");

    let products = vec![
        // Product 1: Cust A, with parts from Tech A
        make_raw_product("p0000000-0000-0000-0000-000000000001", "Laptop Alpha", "MOD-A", "S-001",
            "c0000000-0000-0000-0000-000000000001", "Alice",
            vec!["b0000000-0000-0000-0000-000000000001"], "2024-06-01T00:00:00Z"),
        // Product 2: Cust A, with parts from Tech A and Tech B
        make_raw_product("p0000000-0000-0000-0000-000000000002", "Laptop Beta", "MOD-B", "S-002",
            "c0000000-0000-0000-0000-000000000001", "Alice",
            vec!["b0000000-0000-0000-0000-000000000001", "b0000000-0000-0000-0000-000000000002"], "2024-06-02T00:00:00Z"),
        // Product 3: Cust B, with parts from Tech B only
        make_raw_product("p0000000-0000-0000-0000-000000000003", "Desktop Gamma", "MOD-C", "S-003",
            "c0000000-0000-0000-0000-000000000002", "Bob",
            vec!["b0000000-0000-0000-0000-000000000002"], "2024-06-03T00:00:00Z"),
        // Product 4: Cust B, with no parts yet (new registration)
        make_raw_product("p0000000-0000-0000-0000-000000000004", "Desktop Delta", "MOD-D", "S-004",
            "c0000000-0000-0000-0000-000000000002", "Bob",
            vec![], "2024-06-04T00:00:00Z"),
    ];

    let query = default_products_query();

    // Admin: sees all 4
    let (a_items, _) = products::list_products(&products, "Admin", admin, &query);
    assert_eq!(a_items.len(), 4, "Admin should see all 4 products");

    // Cust A: sees only own products (1 and 2)
    let (ca_items, _) = products::list_products(&products, "Customer", cust_a, &query);
    assert_eq!(ca_items.len(), 2);
    let ca_names: Vec<&str> = ca_items.iter().map(|p| p.product_name.as_str()).collect();
    assert!(ca_names.contains(&"Laptop Alpha"));
    assert!(ca_names.contains(&"Laptop Beta"));

    // Cust B: sees own products (3 and 4)
    let (cb_items, _) = products::list_products(&products, "Customer", cust_b, &query);
    assert_eq!(cb_items.len(), 2);
    assert!(cb_items.iter().any(|p| p.product_name == "Desktop Gamma"));
    assert!(cb_items.iter().any(|p| p.product_name == "Desktop Delta"));

    // Tech A: sees products 1 and 2 (has parts there), NOT 3 or 4
    let (t1_items, _) = products::list_products(&products, "Technician", tech_a, &query);
    assert_eq!(t1_items.len(), 2, "Tech A should see 2 products with their parts");
    let t1_names: Vec<&str> = t1_items.iter().map(|p| p.product_name.as_str()).collect();
    assert!(t1_names.contains(&"Laptop Alpha"));
    assert!(t1_names.contains(&"Laptop Beta"));
    assert!(!t1_names.contains(&"Desktop Gamma"));
    assert!(!t1_names.contains(&"Desktop Delta"));

    // Tech B: sees products 2 and 3 (has parts there), NOT 1 or 4
    let (t2_items, _) = products::list_products(&products, "Technician", tech_b, &query);
    assert_eq!(t2_items.len(), 2, "Tech B should see 2 products with their parts");

    // Tech C: has no parts in any product → sees nothing
    let (t3_items, _) = products::list_products(&products, "Technician", tech_c, &query);
    assert!(t3_items.is_empty(), "Tech C with no parts should see nothing");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 5 — Parts pagination edge cases
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_parts_pagination_edge_cases() {
    let _db = seeded_db().await;
    let admin = u("a0000000-0000-0000-0000-000000000000");

    // 7 parts, all approved, assigned to same tech
    let parts: Vec<RawPart> = (0..7).map(|i| {
        make_raw_part(
            &format!("{i}0000000-0000-0000-0000-000000000000"),
            &format!("PN-{:02}", i + 1),
            &format!("SN-{:02}", i + 1),
            "approved", "Battery", "New",
            Some("b0000000-0000-0000-0000-000000000001"), None, None, None,
            &format!("2024-01-{:02}T00:00:00Z", i + 1),
        )
    }).collect();

    // Page 1, limit 3
    let q1 = ListPartsQuery { page: Some(1), limit: Some(3), ..default_parts_query() };
    let (items1, meta1) = parts::list_parts(&parts, "Admin", admin, &q1);
    assert_eq!(items1.len(), 3);
    assert_eq!(meta1.total_records, 7);
    assert_eq!(meta1.total_pages, 3);
    assert!(meta1.has_next);

    // Page 2, limit 3
    let q2 = ListPartsQuery { page: Some(2), limit: Some(3), ..default_parts_query() };
    let (items2, meta2) = parts::list_parts(&parts, "Admin", admin, &q2);
    assert_eq!(items2.len(), 3);
    assert!(!meta2.has_next);

    // Page 3, limit 3 (last page, 1 item)
    let q3 = ListPartsQuery { page: Some(3), limit: Some(3), ..default_parts_query() };
    let (items3, meta3) = parts::list_parts(&parts, "Admin", admin, &q3);
    assert_eq!(items3.len(), 1);
    assert!(!meta3.has_next);

    // Page 4, limit 3 (beyond range → empty)
    let q4 = ListPartsQuery { page: Some(4), limit: Some(3), ..default_parts_query() };
    let (items4, _) = parts::list_parts(&parts, "Admin", admin, &q4);
    assert!(items4.is_empty(), "Page 4 should be empty with 7 total items at limit 3");

    // Page 0 → clamped to 1
    let q5 = ListPartsQuery { page: Some(0), limit: Some(3), ..default_parts_query() };
    let (items5, meta5) = parts::list_parts(&parts, "Admin", admin, &q5);
    assert_eq!(items5.len(), 3, "Page 0 should be clamped to page 1");
    assert_eq!(meta5.current_page, 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 6 — Part filtering combinations
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_part_filter_combinations() {
    let _db = seeded_db().await;
    let admin = u("a0000000-0000-0000-0000-000000000000");

    let parts = vec![
        make_raw_part("10000000-0000-0000-0000-000000000000", "BAT-001", "S-001", "pending", "Battery", "New", Some("b0000000-0000-0000-0000-000000000001"), None, None, None, "2024-01-01T00:00:00Z"),
        make_raw_part("20000000-0000-0000-0000-000000000000", "SCR-001", "S-002", "approved", "Screen", "Used", Some("b0000000-0000-0000-0000-000000000001"), None, Some("p0000000-0000-0000-0000-000000000001"), Some("Laptop A"), "2024-01-02T00:00:00Z"),
        make_raw_part("30000000-0000-0000-0000-000000000000", "CHG-001", "S-003", "approved", "Charger", "New", Some("b0000000-0000-0000-0000-000000000002"), None, None, None, "2024-01-03T00:00:00Z"),
        make_raw_part("40000000-0000-0000-0000-000000000000", "BAT-002", "S-004", "denied", "Battery", "New", Some("b0000000-0000-0000-0000-000000000003"), None, None, None, "2024-01-04T00:00:00Z"),
    ];

    // Filter: approval_status = approved → 2 results
    let q_approved = ListPartsQuery { approval_status: Some("approved".to_string()), ..default_parts_query() };
    let (approved, _) = parts::list_parts(&parts, "Admin", admin, &q_approved);
    assert_eq!(approved.len(), 2);

    // Filter: approval_status = pending → 1 result
    let q_pending = ListPartsQuery { approval_status: Some("pending".to_string()), ..default_parts_query() };
    let (pending, _) = parts::list_parts(&parts, "Admin", admin, &q_pending);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].part_number, "BAT-001");

    // Filter: search "bat" → 2 results (BAT-001, BAT-002)
    let q_search = ListPartsQuery { search: Some("bat".to_string()), ..default_parts_query() };
    let (search_hits, _) = parts::list_parts(&parts, "Admin", admin, &q_search);
    assert_eq!(search_hits.len(), 2);

    // Combined: approved + search "cha" → 1 result (CHG-001)
    let q_combined = ListPartsQuery {
        approval_status: Some("approved".to_string()),
        search: Some("cha".to_string()),
        ..default_parts_query()
    };
    let (combined, _) = parts::list_parts(&parts, "Admin", admin, &q_combined);
    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0].part_number, "CHG-001");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 7 — Product detail aggregation (parts rolled up correctly)
// ═══════════════════════════════════════════════════════════════════════════════
#[tokio::test]
async fn integration_product_detail_with_parts_rollup() {
    let _db = seeded_db().await;
    let cust = u("c0000000-0000-0000-0000-000000000001");

    let product = make_raw_product(
        "p0000000-0000-0000-0000-000000000001", "Laptop Z", "MOD-Z", "SN-ZZZ",
        "c0000000-0000-0000-0000-000000000001", "Alice",
        vec![
            "b0000000-0000-0000-0000-000000000001",  // part 0
            "b0000000-0000-0000-0000-000000000001",  // part 1 (same tech)
            "b0000000-0000-0000-0000-000000000002",  // part 2 (different tech)
        ],
        "2024-06-01T00:00:00Z",
    );

    // Customer can see detail of their own product
    let detail = products::get_product_detail(&product, "Customer", cust);
    assert!(detail.is_ok());
    let d = detail.unwrap();
    assert_eq!(d.parts.len(), 3, "Should roll up all 3 parts");
    assert_eq!(d.product_id, u("p0000000-0000-0000-0000-000000000001"));
    assert_eq!(d.customer_name, "Alice");

    // Verify parts approval statuses
    for part_item in &d.parts {
        assert_eq!(part_item.approval_status, "approved");
        assert_eq!(part_item.product_name.as_deref(), Some("Laptop Z"));
    }
}
