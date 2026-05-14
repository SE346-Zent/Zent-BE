use crate::model::responses::work_orders::details_response::WorkOrderDetails;
use crate::entities::{work_orders, products, work_order_symptoms, work_order_statuses};
use crate::core::lookup_tables::LookupTables;

pub fn map_to_details(
    wo: work_orders::Model,
    product: Option<products::Model>,
    symptom: Option<work_order_symptoms::Model>,
    status: Option<work_order_statuses::Model>,
) -> WorkOrderDetails {
    WorkOrderDetails {
        id: wo.id,
        work_order_number: wo.work_order_number,
        technician_id: wo.technician_id,
        status: status.map(|s| s.name).unwrap_or_else(|| "Unknown".to_string()),
        customer_id: wo.customer_id,
        customer_name: format!("{} {}", wo.first_name, wo.last_name),
        product_id: wo.product_id,
        product_name: product.map(|p| p.product_name).unwrap_or_else(|| "Unknown Product".to_string()),
        reference_ticket_id: wo.reference_ticket_id,
        symptom_name: symptom.map(|s| s.name).unwrap_or_else(|| "General Service".to_string()),
        description: wo.description,
        first_name: wo.first_name,
        last_name: wo.last_name,
        email: wo.email,
        phone_number: wo.phone_number,
        country: wo.country,
        province: wo.province,
        city: wo.city,
        address: wo.address,
        building: wo.building,
        appointment: wo.appointment,
        created_at: wo.created_at,
        updated_at: wo.updated_at,
    }
}

pub fn decide_get_details(
    wo: work_orders::Model,
    product: Option<products::Model>,
    symptom: Option<work_order_symptoms::Model>,
    lookup_tables: &LookupTables,
) -> WorkOrderDetails {
    let status_name = lookup_tables.work_order_statuses.get(&wo.work_order_status_id).cloned();
    let status = status_name.map(|name| work_order_statuses::Model {
        id: wo.work_order_status_id,
        name,
    });
    map_to_details(wo, product, symptom, status)
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
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
        let wo = dummy_work_order();
        let prod = dummy_product();
        let symp = dummy_symptom();
        let status = work_order_statuses::Model { id: 1, name: "Pending".to_string() };

        let details = map_to_details(wo, Some(prod), Some(symp), Some(status));
        assert_eq!(details.work_order_number, "WO-999");
        assert_eq!(details.customer_name, "Jane Smith");
        assert_eq!(details.product_name, "Super Widget");
        assert_eq!(details.symptom_name, "Does not turn on");
        assert_eq!(details.status, "Pending");
        assert!(details.technician_id.is_none());
    }

    #[test]
    fn test_map_to_details_missing_relations() {
        let wo = dummy_work_order();
        let details = map_to_details(wo, None, None, None);
        assert_eq!(details.product_name, "Unknown Product");
        assert_eq!(details.symptom_name, "General Service");
        assert_eq!(details.status, "Unknown");
    }

    #[test]
    fn test_decide_get_details_with_lut() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());

        let wo = dummy_work_order();
        let details = decide_get_details(wo, None, None, &luts);

        assert_eq!(details.status, "Pending");
        assert_eq!(details.product_name, "Unknown Product");
        assert_eq!(details.symptom_name, "General Service");
    }
}
