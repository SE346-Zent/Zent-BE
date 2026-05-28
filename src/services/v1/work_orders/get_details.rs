use crate::model::responses::work_orders::details_response::WorkOrderDetails;
use crate::entities::{work_orders, products, work_order_symptoms, work_order_statuses};
use crate::core::lookup_tables::LookupTables;

/// Transform a database work order model and its related entities into a comprehensive details view.
///
/// This function provides a flattened view of the work order, product, symptom, 
/// and status information, handling name concatenation and providing fallback 
/// values for missing relations.

pub fn map_to_details(
    work_order: work_orders::Model,
    product: Option<products::Model>,
    symptom: Option<work_order_symptoms::Model>,
    status: Option<work_order_statuses::Model>,
) -> WorkOrderDetails {
    WorkOrderDetails {
        id: work_order.id,
        work_order_number: work_order.work_order_number,
        technician_id: work_order.technician_id,
        status: status.map(|s| s.name).unwrap_or_else(|| "Unknown".to_string()),
        customer_id: work_order.customer_id,
        customer_name: format!("{} {}", work_order.first_name, work_order.last_name),
        product_id: work_order.product_id,
        product_name: product.map(|p| p.product_name).unwrap_or_else(|| "Unknown Product".to_string()),
        reference_ticket_id: work_order.reference_ticket_id,
        symptom_name: symptom.map(|s| s.name).unwrap_or_else(|| "General Service".to_string()),
        description: work_order.description,
        first_name: work_order.first_name,
        last_name: work_order.last_name,
        email: work_order.email,
        phone_number: work_order.phone_number,
        country: work_order.country,
        province: work_order.province,
        city: work_order.city,
        address: work_order.address,
        building: work_order.building,
        appointment: crate::utils::time::to_utc7_string(work_order.appointment),
        created_at: crate::utils::time::to_utc7_string(work_order.created_at),
        updated_at: crate::utils::time::to_utc7_string(work_order.updated_at),
    }
}

/// Prepare the detailed view of a single work order by resolving status data from lookup tables.
///
/// # Arguments
/// * `work_order` - The database model for the work order.
/// * `product` - Optional database model for the associated product.
/// * `symptom` - Optional database model for the reported symptom.
/// * `lookup_tables` - Shared reference data for resolving status names.

pub fn decide_get_details(
    work_order: work_orders::Model,
    product: Option<products::Model>,
    symptom: Option<work_order_symptoms::Model>,
    lookup_tables: &LookupTables,
) -> WorkOrderDetails {
    let status_name = lookup_tables.work_order_statuses.get(&work_order.work_order_status_id).cloned();
    let status = status_name.map(|name| work_order_statuses::Model {
        id: work_order.work_order_status_id,
        name,
    });
    map_to_details(work_order, product, symptom, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn dummy_work_order() -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 1,
            customer_id: Uuid::new_v4(),
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "Broken thing".to_string(),
            first_name: "Jane".to_string(),
            last_name: "Smith".to_string(),
            email: None,
            phone_number: None,
            country: "Canada".to_string(),
            province: "ON".to_string(),
            city: "Toronto".to_string(),
            address: "123 Main St".to_string(),
            building: None,
            appointment: Utc::now(),
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "WO-999".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    fn dummy_product() -> products::Model {
        products::Model {
            id: Uuid::new_v4(),
            product_model_code: "SW-100".to_string(),
            customer_id: Uuid::new_v4(),
            product_name: "Super Widget".to_string(),
            serial_number: "SN123".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    fn dummy_symptom() -> work_order_symptoms::Model {
        work_order_symptoms::Model {
            id: 1,
            name: "Does not turn on".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_map_to_details_full() {
        let work_order = dummy_work_order();
        let product = dummy_product();
        let symptom = dummy_symptom();
        let status = work_order_statuses::Model { id: 1, name: "Pending".to_string() };

        let details = map_to_details(work_order, Some(product), Some(symptom), Some(status));
        assert_eq!(details.work_order_number, "WO-999");
        assert_eq!(details.customer_name, "Jane Smith");
        assert_eq!(details.product_name, "Super Widget");
        assert_eq!(details.symptom_name, "Does not turn on");
        assert_eq!(details.status, "Pending");
        assert!(details.technician_id.is_none());
    }

    #[test]
    fn test_map_to_details_missing_relations() {
        let work_order = dummy_work_order();
        let details = map_to_details(work_order, None, None, None);
        assert_eq!(details.product_name, "Unknown Product");
        assert_eq!(details.symptom_name, "General Service");
        assert_eq!(details.status, "Unknown");
    }

    #[test]
    fn test_decide_get_details_with_lut() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());

        let work_order = dummy_work_order();
        let details = decide_get_details(work_order, None, None, &luts);

        assert_eq!(details.status, "Pending");
        assert_eq!(details.product_name, "Unknown Product");
        assert_eq!(details.symptom_name, "General Service");
    }
}
