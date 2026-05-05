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
        state: wo.state,
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
