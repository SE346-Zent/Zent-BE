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
