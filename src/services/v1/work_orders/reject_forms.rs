use crate::{
    entities::{work_orders, work_order_reject_forms, users},
    model::responses::work_orders::reject_form_list_response::RejectFormListItem,
};

/// Pure logic: maps a joined row (work_order, reject_form, technician) into a list item.
pub fn map_to_reject_form_list_item(
    wo: work_orders::Model,
    rf: work_order_reject_forms::Model,
    technician: Option<users::Model>,
) -> RejectFormListItem {
    RejectFormListItem {
        reject_form_id: rf.id,
        work_order_id: wo.id,
        work_order_number: wo.work_order_number,
        technician_name: technician.map(|u| u.full_name).unwrap_or_else(|| "Unknown".to_string()),
        customer_name: format!("{} {}", wo.first_name, wo.last_name),
        reason: rf.reason,
        approved: rf.approved,
        created_at: rf.created_at,
    }
}
