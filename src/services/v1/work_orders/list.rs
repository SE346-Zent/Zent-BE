use crate::model::responses::work_orders::list_response::WorkOrderListItem;
use crate::model::responses::pagination::PaginationResponse;
use crate::model::requests::pagination::PaginationRequest;
use crate::entities::{work_orders, products, work_order_symptoms, work_order_statuses};
use crate::core::lookup_tables::LookupTables;
use uuid::Uuid;

/// Transform a database work order model and its related entities into a high-level summary for list displays.
///
/// This function handles the concatenation of names and addresses, and provides 
/// fallback values if related entities like products or statuses are missing.

pub fn map_to_list_item(
    work_order: work_orders::Model,
    product: Option<products::Model>,
    _symptom: Option<work_order_symptoms::Model>,
    status: Option<work_order_statuses::Model>,
    has_rating: bool,
) -> WorkOrderListItem {
    WorkOrderListItem {
        id: work_order.id,
        work_order_num: work_order.work_order_number,
        status: status.map(|s| s.name).unwrap_or_else(|| "Unknown".to_string()),
        customer_name: format!("{} {}", work_order.first_name, work_order.last_name),
        product_name: product.map(|p| p.product_name).unwrap_or_else(|| "Unknown Product".to_string()),
        address: format!("{}, {}, {}", work_order.address, work_order.city, work_order.province),
        appointment: Some(work_order.appointment),
        has_rating,
        created_at: work_order.created_at,
    }
}

/// Apply pagination and lookup table data to a list of work order models.
///
/// This function converts raw database records (tuples of work order, product, and symptom)
/// into a paginated response containing human-readable `WorkOrderListItem` objects.

pub fn decide_list(
    work_order_tuples: Vec<(work_orders::Model, Option<products::Model>, Option<work_order_symptoms::Model>)>,
    lookup_tables: &LookupTables,
    pagination: &PaginationRequest,
    total_records: u64,
    rated_work_order_ids: &std::collections::HashSet<Uuid>,
) -> (Vec<WorkOrderListItem>, PaginationResponse) {
    let list_items = work_order_tuples
        .into_iter()
        .map(|(work_order, product, symptom)| {
            let status_name = lookup_tables.work_order_statuses.get(&work_order.work_order_status_id).cloned();
            let status = status_name.map(|name| work_order_statuses::Model {
                id: work_order.work_order_status_id,
                name,
            });
            let has_rating = rated_work_order_ids.contains(&work_order.id);
            map_to_list_item(work_order, product, symptom, status, has_rating)
        })
        .collect();

    (
        list_items,
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

    #[test]
    fn test_map_to_list_item_full() {
        let work_order = dummy_work_order();
        let product = dummy_product();
        let status = work_order_statuses::Model { id: 1, name: "Pending".to_string() };

        let item = map_to_list_item(work_order, Some(product), None, Some(status), false);
        assert_eq!(item.work_order_num, "WO-123");
        assert_eq!(item.customer_name, "John Doe");
        assert_eq!(item.product_name, "Super Widget");
        assert_eq!(item.address, "123 Main St, Toronto, ON");
        assert_eq!(item.status, "Pending");
        assert!(!item.has_rating);
    }

    #[test]
    fn test_map_to_list_item_missing_relations() {
        let work_order = dummy_work_order();
        let item = map_to_list_item(work_order, None, None, None, false);
        assert_eq!(item.product_name, "Unknown Product");
        assert_eq!(item.status, "Unknown");
    }

    #[test]
    fn test_decide_list_pagination() {
        let mut luts = LookupTables::empty();
        luts.work_order_statuses.insert(1, "Pending".to_string());

        let work_order = dummy_work_order();
        let models = vec![(work_order, None, None)];

        let req = PaginationRequest { limit: 10, page: 2 };
        let rated_ids = std::collections::HashSet::new();
        let (items, pag) = decide_list(models, &luts, &req, 100, &rated_ids);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status, "Pending");
        assert_eq!(pag.total_pages, 10);
        assert_eq!(pag.total_records, 100);
        assert_eq!(pag.current_page, 2);
    }
}
