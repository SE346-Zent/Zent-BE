//! Integration tests for the inventory domain.
//!
//! Tests the service layer directly with mock-assembled entity structs.
//! Exercises the pure service logic by calling service functions directly
//! with the new PartWithRelations / ProductWithRelations types.

use uuid::Uuid;


use zent_be::services::v1::inventory::get_product::{self, ProductWithRelations};
use zent_be::services::v1::inventory::get_product::{PartInProduct as DetailPartInProduct};
use zent_be::services::v1::inventory::accept_part;
use zent_be::services::v1::inventory::deny_part;
use zent_be::entities::{parts, part_catalog, part_conditions, products, product_models};

fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
fn t() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEST 1 — Approval state machine with audit trail
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
    assert_eq!(r.unwrap().approval_audit_model.action.unwrap(), "approved");

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
    assert_eq!(r.unwrap().denial_audit_model.action.unwrap(), "denied");

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
// TEST 2 — Product detail aggregation (parts rolled up correctly)
// ═══════════════════════════════════════════════════════════════════════════════
#[test]
fn integration_product_detail_with_parts_rollup() {
    let cust = u("c0000000-0000-0000-0000-000000000001");
    let now = t();

    let product_with_relations = ProductWithRelations {
        product_record: products::Model {
            id: u("p0000000-0000-0000-0000-000000000001"),
            product_model_code: "MOD-Z".to_string(),
            customer_id: cust,
            product_name: "Laptop Z".to_string(),
            serial_number: "SN-ZZZ".to_string(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        model_definition: product_models::Model {
            model_code: "MOD-Z".to_string(),
            model_name: "Model MOD-Z".to_string(),
            description: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        },
        installed_parts: vec![
            DetailPartInProduct { // part 0
                part_record: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000001"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P0".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog_definition: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P0".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                physical_condition: part_conditions::Model { id: 1, name: "New".to_string() },
                approval_status: "approved".to_string(),
                registering_technician_id: Some(u("b0000000-0000-0000-0000-000000000001")),
            },
            DetailPartInProduct { // part 1 (same tech)
                part_record: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000002"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P1".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog_definition: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P1".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                physical_condition: part_conditions::Model { id: 1, name: "Used".to_string() },
                approval_status: "approved".to_string(),
                registering_technician_id: Some(u("b0000000-0000-0000-0000-000000000001")),
            },
            DetailPartInProduct { // part 2 (different tech)
                part_record: parts::Model {
                    id: u("b0000000-0000-0000-0000-000000000003"),
                    part_catalog_id: Uuid::new_v4(),
                    product_id: Some(u("p0000000-0000-0000-0000-000000000001")),
                    serial_number: "SN-Z-P2".to_string(),
                    part_condition_id: 1,
                    manufactured_date: now,
                    installation_date: None, removal_date: None, scrapped_date: None,
                    created_at: now, updated_at: now, deleted_at: None,
                },
                catalog_definition: part_catalog::Model {
                    id: Uuid::new_v4(),
                    part_number: "PN-Z-P2".to_string(),
                    part_types_id: 1, mfg_number: "MFG".to_string(), description: None,
                    part_mfg_status: 1, created_at: now, updated_at: now, deleted_at: None,
                },
                physical_condition: part_conditions::Model { id: 1, name: "New".to_string() },
                approval_status: "approved".to_string(),
                registering_technician_id: Some(u("b0000000-0000-0000-0000-000000000002")),
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
