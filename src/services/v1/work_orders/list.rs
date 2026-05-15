use crate::model::responses::work_orders::list_response::WorkOrderListItem;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::requests::pagination::PaginationRequest;
use crate::entities::{work_orders, products, work_order_symptoms, work_order_statuses};
use crate::core::lookup_tables::LookupTables;

pub fn map_to_list_item(
    wo: work_orders::Model,
    product: Option<products::Model>,
    _symptom: Option<work_order_symptoms::Model>,
    status: Option<work_order_statuses::Model>,
) -> WorkOrderListItem {
    WorkOrderListItem {
        id: wo.id,
        work_order_num: wo.work_order_number,
        status: status.map(|s| s.name).unwrap_or_else(|| "Unknown".to_string()),
        customer_name: format!("{} {}", wo.first_name, wo.last_name),
        product_name: product.map(|p| p.product_name).unwrap_or_else(|| "Unknown Product".to_string()),
        address: format!("{}, {}, {}", wo.address, wo.city, wo.province),
        appointment: Some(wo.appointment),
        created_at: wo.created_at,
    }
}

pub fn decide_list(
    models: Vec<(work_orders::Model, Option<products::Model>, Option<work_order_symptoms::Model>)>,
    lookup_tables: &LookupTables,
    pagination: &PaginationRequest,
    total_records: u64,
) -> (Vec<WorkOrderListItem>, PaginationResponse) {
    let data = models
        .into_iter()
        .map(|(wo, product, symptom)| {
            let status_name = lookup_tables.work_order_statuses.get(&wo.work_order_status_id).cloned();
            let status = status_name.map(|name| work_order_statuses::Model {
                id: wo.work_order_status_id,
                name,
            });
            map_to_list_item(wo, product, symptom, status)
        })
        .collect();

    (
        data,
        PaginationResponse::new(pagination.limit, pagination.page, total_records),
    )
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
            description: "".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: None,
            phone_number: None,
            country: "".to_string(),
            province: "ON".to_string(),
            city: "Toronto".to_string(),
            address: "123 Main St".to_string(),
            building: None,
            appointment: Utc::now(),
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "WO-123".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            customer_complaint: None,
            customer_complaint_at: None,
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

    #[test]
    fn test_map_to_list_item_full() {
        let wo = dummy_work_order();
        let prod = dummy_product();
        let status = work_order_statuses::Model { id: 1, name: "Pending".to_string() };

        let item = map_to_list_item(wo, Some(prod), None, Some(status));
        assert_eq!(item.work_order_num, "WO-123");
        assert_eq!(item.customer_name, "John Doe");
        assert_eq!(item.product_name, "Super Widget");
        assert_eq!(item.address, "123 Main St, Toronto, ON");
        assert_eq!(item.status, "Pending");
    }

    #[test]
    fn test_map_to_list_item_missing_relations() {
        let wo = dummy_work_order();
        let item = map_to_list_item(wo, None, None, None);
        assert_eq!(item.product_name, "Unknown Product");
        assert_eq!(item.status, "Unknown");
    }

    #[test]
    fn test_decide_list_pagination() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());

        let wo = dummy_work_order();
        let models = vec![(wo, None, None)];

        let req = PaginationRequest { limit: 10, page: 2 };
        let (items, pag) = decide_list(models, &luts, &req, 100);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "Pending");
        assert_eq!(pag.total_pages, 10);
        assert_eq!(pag.total_records, 100);
        assert_eq!(pag.current_page, 2);
    }
}
