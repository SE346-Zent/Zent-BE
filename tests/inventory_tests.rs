//! Integration tests for the inventory domain.
//!
//! Tests the service layer directly with mock-assembled entity structs.
//! Since handlers are stubbed with `unimplemented!()`, these tests
//! exercise the pure service logic by calling service functions directly
//! with the new PartEntry / ProductEntry / PartWithRelations / ProductWithRelations types.

use uuid::Uuid;


use zent_be::services::v1::inventory::list_parts::{self, PartEntry};
use zent_be::services::v1::inventory::list_products::{self, ProductEntry, PartInProduct as ListPartInProduct};
use zent_be::services::v1::inventory::get_product::{PartInProduct as DetailPartInProduct};
use zent_be::services::v1::inventory::get_product::{self, ProductWithRelations};
use zent_be::services::v1::inventory::accept_part;
use zent_be::services::v1::inventory::deny_part;
use zent_be::services::v1::inventory::register_product;
use zent_be::model::requests::inventory::list_parts_query::ListPartsQuery;
use zent_be::model::requests::inventory::list_products_query::ListProductsQuery;
use zent_be::model::requests::inventory::register_product_request::RegisterProductRequest;
use zent_be::entities::{parts, part_catalog, part_conditions, products, product_models};

fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
fn t() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers — build PartEntry / ProductEntry / queries
// ─────────────────────────────────────────────────────────────────────────────

fn make_part_entry(
    part_id: &str, part_number: &str, serial: &str, approval: &str,
    _type_name: &str, condition_name: &str, tech_id: Option<&str>, cust_id: Option<&str>,
    product_id: Option<&str>, _product_name: Option<&str>,
) -> PartEntry {
    let now = t();
    PartEntry {
        part: parts::Model {
            id: u(part_id),
            part_catalog_id: Uuid::new_v4(),
            product_id: product_id.map(|s| u(s)),
            serial_number: serial.to_string(),
            part_condition_id: 1,
            manufactured_date: now,
            installation_date: None,
            removal_date: None,
            scrapped_date: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        catalog: part_catalog::Model {
            id: Uuid::new_v4(),
            part_number: part_number.to_string(),
            part_types_id: 1,
            mfg_number: "MFG-1".to_string(),
            description: None,
            part_mfg_status: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        condition: part_conditions::Model {
            id: 1,
            name: condition_name.to_string(),
        },
        product: product_id.map(|pid| products::Model {
            id: u(pid),
            product_model_code: "MODEL-A".to_string(),
            customer_id: cust_id.map_or(Uuid::nil(), |s| u(s)),
            product_name: _product_name.unwrap_or("Product").to_string(),
            serial_number: format!("SN-{}", pid),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }),
        status: approval.to_string(),
        denial_reason: None,
        customer_id: cust_id.map(|s| u(s)),
        technician_id: tech_id.map(|s| u(s)),
    }
}

fn make_product_entry(
    id: &str, name: &str, model: &str, serial: &str, cust_id: &str,
    _cust_name: &str, part_ids: &[&str],
) -> ProductEntry {
    let now = t();
    ProductEntry {
        product: products::Model {
            id: u(id),
            product_model_code: model.to_string(),
            customer_id: u(cust_id),
            product_name: name.to_string(),
            serial_number: serial.to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        model: product_models::Model {
            model_code: model.to_string(),
            model_name: format!("Model {}", model),
            description: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        parts: part_ids.iter().enumerate().map(|(i, tid)| ListPartInProduct {
            part: parts::Model {
                id: Uuid::new_v4(),
                part_catalog_id: Uuid::new_v4(),
                product_id: Some(u(id)),
                serial_number: format!("SN-{}-P{}", id, i),
                part_condition_id: 1,
                manufactured_date: now,
                installation_date: None,
                removal_date: None,
                scrapped_date: None,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            catalog: part_catalog::Model {
                id: Uuid::new_v4(),
                part_number: format!("PN-{}-P{}", id, i),
                part_types_id: 1,
                mfg_number: "MFG-1".to_string(),
                description: None,
                part_mfg_status: 1,
                created_at: now,
                updated_at: now,
                deleted_at: None,
            },
            condition: part_conditions::Model {
                id: 1,
                name: "New".to_string(),
            },
            technician_id: if !tid.is_empty() { Some(u(tid)) } else { None },
        }).collect(),
    }
}

fn default_parts_query() -> ListPartsQuery {
    ListPartsQuery {
        model_code: None,
        part_type_id: None,
        approval_status: None,
        search: None,
        page: None,
        limit: None,
        sort_by: None,
        sort_order: None,
    }
}

fn default_products_query() -> ListProductsQuery {
    ListProductsQuery {
        model_code: None,
        search: None,
        page: None,
        limit: None,
        sort_by: None,
        sort_order: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 1 — Role-based access matrix (10 parts, 5 users)
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_role_access_matrix_10_parts_5_users() {
    let admin = u("a0000000-0000-0000-0000-000000000001");
    let tech_a = u("b0000000-0000-0000-0000-000000000001");
    let tech_b = u("b0000000-0000-0000-0000-000000000002");
    let cust_a = u("c0000000-0000-0000-0000-000000000001");
    let cust_b = u("c0000000-0000-0000-0000-000000000002");

    // 10 parts with varying ownership / approval status
    let parts = vec![
        make_part_entry("p0000000-0000-0000-0000-000000000001", "PN-001", "SN-001", "approved", "Battery", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000002", "PN-002", "SN-002", "approved", "Battery", "New", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000002"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000003", "PN-003", "SN-003", "pending", "Display", "Used", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000004", "PN-004", "SN-004", "denied", "Charger", "New", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000005", "PN-005", "SN-005", "approved", "Audio", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000002"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000006", "PN-006", "SN-006", "approved", "Battery", "Used", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000007", "PN-007", "SN-007", "pending", "Display", "New", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000002"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000008", "PN-008", "SN-008", "approved", "Charger", "New", Some("b0000000-0000-0000-0000-000000000002"), Some("c0000000-0000-0000-0000-000000000002"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000009", "PN-009", "SN-009", "denied", "Audio", "Used", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000010", "PN-010", "SN-010", "pending", "Battery", "New", Some("b0000000-0000-0000-0000-000000000001"), Some("c0000000-0000-0000-0000-000000000002"), None, None),
    ];

    let q = default_parts_query();

    // Admin sees everything
    let (admin_view, _) = list_parts::list_parts(&parts, "Admin", admin, &q);
    assert_eq!(admin_view.len(), 10);

    // Tech A sees their own parts regardless of status
    let (tech_a_view, _) = list_parts::list_parts(&parts, "Technician", tech_a, &q);
    assert_eq!(tech_a_view.len(), 6);  // parts 1,3,5,6,9,10

    // Tech B sees their own parts
    let (tech_b_view, _) = list_parts::list_parts(&parts, "Technician", tech_b, &q);
    assert_eq!(tech_b_view.len(), 4);  // parts 2,4,7,8

    // Customer A only sees approved parts they own
    let (cust_a_view, _) = list_parts::list_parts(&parts, "Customer", cust_a, &q);
    assert_eq!(cust_a_view.len(), 2);  // parts 1,6
    for item in &cust_a_view {
        assert_eq!(item.approval_status, "approved");
    }

    // Customer B only sees approved parts they own
    let (cust_b_view, _) = list_parts::list_parts(&parts, "Customer", cust_b, &q);
    assert_eq!(cust_b_view.len(), 2);  // parts 2,8
    for item in &cust_b_view {
        assert_eq!(item.approval_status, "approved");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 2 — Approval state machine with audit trail
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_approval_state_machine_with_audit() {
    let admin_id = u("a0000000-0000-0000-0000-000000000001");
    let now = chrono::Utc::now();

    // Accept pending → succeeds
    let r = accept_part::decide_accept_part(
        u("f0000000-0000-0000-0000-000000000001"),
        admin_id, "pending", now,
    );
    assert!(r.is_ok());
    assert_eq!(r.unwrap().audit.action.unwrap(), "approved");

    // Accept non-pending → fails
    let r = accept_part::decide_accept_part(
        u("f0000000-0000-0000-0000-000000000002"),
        admin_id, "approved", now,
    );
    assert!(r.is_err());

    // Deny pending with valid reason → succeeds
    let r = deny_part::decide_deny_part(
        u("f0000000-0000-0000-0000-000000000003"),
        admin_id, "pending",
        "Part does not meet quality standards due to visible damage",
        now,
    );
    assert!(r.is_ok());
    assert_eq!(r.unwrap().audit.action.unwrap(), "denied");

    // Deny with short reason → fails
    let r = deny_part::decide_deny_part(
        u("f0000000-0000-0000-0000-000000000004"),
        admin_id, "pending", "short", now,
    );
    assert!(r.is_err());

    // Deny non-pending → fails
    let r = deny_part::decide_deny_part(
        u("f0000000-0000-0000-0000-000000000005"),
        admin_id, "denied",
        "This part was already denied by the approver", now,
    );
    assert!(r.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 3 — Product registration logic
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_product_registration_complex() {
    let user_id = u("c0000000-0000-0000-0000-000000000001");
    let now = chrono::Utc::now();

    // Successful first registration (no existing product)
    let req = RegisterProductRequest {
        serial_number: "SN-VALID-123".to_string(),
        country: "Vietnam".to_string(),
        province: "Hanoi".to_string(),
        city: "Cau Giay".to_string(),
        address: "123 Test Street".to_string(),
        first_name: "John".to_string(),
        last_name: "Doe".to_string(),
        email: "john@example.com".to_string(),
        mobile_phone: "0123456789".to_string(),
        send_email_confirmation: true,
    };

    let result = register_product::decide_register_product(
        &req, user_id, "John Doe",
        Some("MODEL-A".to_string()), Some("Model A".to_string()),
        None, now,
    );
    assert!(result.is_ok());
    let effect = result.unwrap();
    assert!(effect.should_send_email);
    assert_eq!(effect.model_code, "MODEL-A");

    // Serial not in catalog → fails
    let result = register_product::decide_register_product(
        &req, user_id, "John Doe",
        None, None,
        None, now,
    );
    assert!(result.is_err());

    // Re-registration of existing product (existing_product_id is Some)
    let result = register_product::decide_register_product(
        &req, user_id, "John Doe",
        Some("MODEL-A".to_string()), Some("Model A".to_string()),
        Some(u("p1000000-0000-0000-0000-000000000001")), now,
    );
    assert!(result.is_ok());
    let effect = result.unwrap();
    assert!(!effect.should_send_email); // No email on re-registration
    assert_eq!(effect.product_id, u("p1000000-0000-0000-0000-000000000001"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 4 — Product isolation across roles
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_product_isolation_across_roles() {
    let admin = u("a0000000-0000-0000-0000-000000000001");
    let tech = u("b0000000-0000-0000-0000-000000000001");
    let cust = u("c0000000-0000-0000-0000-000000000001");
    let other = u("c0000000-0000-0000-0000-000000000002");

    let products = vec![
        make_product_entry("p0000000-0000-0000-0000-000000000001", "Laptop X", "MOD-X", "SN-XXX", "c0000000-0000-0000-0000-000000000001", "Alice", &["b0000000-0000-0000-0000-000000000001"]),
        make_product_entry("p0000000-0000-0000-0000-000000000002", "Laptop Y", "MOD-Y", "SN-YYY", "c0000000-0000-0000-0000-000000000002", "Bob", &["b0000000-0000-0000-0000-000000000001"]),
        make_product_entry("p0000000-0000-0000-0000-000000000003", "Laptop Z", "MOD-Z", "SN-ZZZ", "c0000000-0000-0000-0000-000000000001", "Alice", &[]),
    ];

    let q = default_products_query();

    // Admin sees all
    let (admin_view, _) = list_products::list_products(&products, "Admin", admin, &q);
    assert_eq!(admin_view.len(), 3);

    // Tech sees only products with their parts
    let (tech_view, _) = list_products::list_products(&products, "Technician", tech, &q);
    assert_eq!(tech_view.len(), 2); // MOD-X, MOD-Y (not MOD-Z — no parts assigned to tech)

    // Customer A sees only their own products
    let (cust_a_view, _) = list_products::list_products(&products, "Customer", cust, &q);
    assert_eq!(cust_a_view.len(), 2); // MOD-X, MOD-Z

    // Customer B sees only their own products
    let (cust_b_view, _) = list_products::list_products(&products, "Customer", other, &q);
    assert_eq!(cust_b_view.len(), 1); // MOD-Y

    // Detail access
    let prod_a = ProductWithRelations {
        product: products[0].product.clone(),
        model: products[0].model.clone(),
        parts: products[0].parts.iter().map(|p| DetailPartInProduct {
            part: p.part.clone(),
            catalog: p.catalog.clone(),
            condition: p.condition.clone(),
            status: "approved".to_string(),
            technician_id: p.technician_id,
        }).collect(),
    };
    assert!(get_product::get_product_detail(&prod_a, "Admin", admin).is_ok());
    assert!(get_product::get_product_detail(&prod_a, "Customer", other).is_err()); // wrong customer
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 5 — Parts pagination edge cases
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_parts_pagination_edge_cases() {
    let admin = u("a0000000-0000-0000-0000-000000000001");
    let mut parts = Vec::new();
    for i in 1..=25 {
        parts.push(make_part_entry(
            &format!("p{:032}", i),
            &format!("PN-{:03}", i),
            &format!("SN-{:03}", i),
            "approved", "Battery", "New",
            None, Some("c0000000-0000-0000-0000-000000000001"),
            None, None,
        ));
    }

    // Default page (page 1, limit 20)
    let q = default_parts_query();
    let (items, meta) = list_parts::list_parts(&parts, "Admin", admin, &q);
    assert_eq!(items.len(), 20);
    assert_eq!(meta.total_records, 25);
    assert_eq!(meta.current_page, 1);
    assert_eq!(meta.total_pages, 2);
    assert!(meta.has_next);

    // Page 2
    let q2 = ListPartsQuery { page: Some(2), limit: Some(20), ..default_parts_query() };
    let (items2, meta2) = list_parts::list_parts(&parts, "Admin", admin, &q2);
    assert_eq!(items2.len(), 5);
    assert_eq!(meta2.current_page, 2);
    assert!(!meta2.has_next);

    // Limit=5, page=3 → items 11-15
    let q3 = ListPartsQuery { page: Some(3), limit: Some(5), ..default_parts_query() };
    let (items3, _) = list_parts::list_parts(&parts, "Admin", admin, &q3);
    assert_eq!(items3.len(), 5);

    // Page beyond range
    let q4 = ListPartsQuery { page: Some(10), limit: Some(20), ..default_parts_query() };
    let (items4, _) = list_parts::list_parts(&parts, "Admin", admin, &q4);
    assert!(items4.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 6 — Part filter combinations
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_part_filter_combinations() {
    let admin = u("a0000000-0000-0000-0000-000000000001");
    let parts = vec![
        make_part_entry("p0000000-0000-0000-0000-000000000001", "PN-001", "SN-001", "approved", "Battery", "New", None, Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000002", "PN-002", "SN-002", "pending", "Battery", "Used", None, Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000003", "PN-003", "SN-003", "denied", "Charger", "New", None, Some("c0000000-0000-0000-0000-000000000001"), None, None),
        make_part_entry("p0000000-0000-0000-0000-000000000004", "PN-CHG-001", "SN-CHG-001", "approved", "Charger", "New", None, Some("c0000000-0000-0000-0000-000000000001"), None, None),
    ];

    // Filter by approval status
    let q_approved = ListPartsQuery { approval_status: Some("approved".to_string()), ..default_parts_query() };
    let (approved, _) = list_parts::list_parts(&parts, "Admin", admin, &q_approved);
    assert_eq!(approved.len(), 2); // PN-001, PN-CHG-001

    // Filter by search term (matches PN-001 and SN-001 → same part)
    let q_search = ListPartsQuery { search: Some("PN-001".to_string()), ..default_parts_query() };
    let (searched, _) = list_parts::list_parts(&parts, "Admin", admin, &q_search);
    assert_eq!(searched.len(), 1);

    // Combined: approved + search "CHG"
    let q_combined = ListPartsQuery {
        approval_status: Some("approved".to_string()),
        search: Some("CHG".to_string()),
        ..default_parts_query()
    };
    let (combined, _) = list_parts::list_parts(&parts, "Admin", admin, &q_combined);
    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0].part_number, "PN-CHG-001");
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 7 — Product detail aggregation (parts rolled up correctly)
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_product_detail_with_parts_rollup() {
    let cust = u("c0000000-0000-0000-0000-000000000001");
    let now = t();

    let product_with_relations = ProductWithRelations {
        product: products::Model {
            id: u("p0000000-0000-0000-0000-000000000001"),
            product_model_code: "MOD-Z".to_string(),
            customer_id: cust,
            product_name: "Laptop Z".to_string(),
            serial_number: "SN-ZZZ".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        model: product_models::Model {
            model_code: "MOD-Z".to_string(),
            model_name: "Model MOD-Z".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        parts: vec![
            DetailPartInProduct { // part 0
                part: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000001"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P0".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P0".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                condition: part_conditions::Model { id: 1, name: "New".to_string() },
                status: "approved".to_string(),
                technician_id: Some(u("b0000000-0000-0000-0000-000000000001")),
            },
            DetailPartInProduct { // part 1 (same tech)
                part: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000002"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P1".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P1".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                condition: part_conditions::Model { id: 1, name: "Used".to_string() },
                status: "approved".to_string(),
                technician_id: Some(u("b0000000-0000-0000-0000-000000000001")),
            },
            DetailPartInProduct { // part 2 (different tech)
                part: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000003"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P2".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P2".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                condition: part_conditions::Model { id: 1, name: "New".to_string() },
                status: "approved".to_string(),
                technician_id: Some(u("b0000000-0000-0000-0000-000000000002")),
            },
        ],
    };

    // Customer can see detail of their own product
    let detail = get_product::get_product_detail(&product_with_relations, "Customer", cust);
    assert!(detail.is_ok());
    let d = detail.unwrap();
    assert_eq!(d.parts.len(), 3, "Should roll up all 3 parts");
    assert_eq!(d.product_id, u("p0000000-0000-0000-0000-000000000001"));
    assert_eq!(d.customer_name, format!("Customer {}", cust));

    // Verify parts approval statuses
    for part_item in &d.parts {
        assert_eq!(part_item.approval_status, "approved");
        assert_eq!(part_item.product_name.as_deref(), Some("Laptop Z"));
    }
}
