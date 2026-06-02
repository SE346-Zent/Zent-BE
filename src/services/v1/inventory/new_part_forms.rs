use crate::model::responses::inventory::new_part_form_list_response::{NewPartFormListItem, NewPartFormListResponse, NewPartFormStatusSummary};
use crate::model::responses::inventory::new_part_form_detail_response::NewPartFormDetailResponse;

/// Pure mapping logic for new part forms list response.
pub fn map_list_response(
    rows: Vec<(crate::entities::new_part_forms::Model, Option<crate::entities::part_types::Model>)>,
    page: u64,
    limit: u64,
) -> (NewPartFormListResponse, u64) {
    let total_records = rows.len() as u64;
    let summary = rows.iter().fold(NewPartFormStatusSummary { pending: 0, approved: 0, rejected: 0 }, |mut acc, (form, _)| {
        match form.status.to_lowercase().as_str() {
            "pending" => acc.pending += 1,
            "approved" => acc.approved += 1,
            "rejected" | "denied" => acc.rejected += 1,
            _ => {}
        }
        acc
    });

    let page_start = ((page - 1) * limit) as usize;
    let page_end = (page * limit) as usize;
    let paged_rows = rows
        .into_iter()
        .skip(page_start)
        .take((page_end.saturating_sub(page_start)) as usize)
        .collect::<Vec<_>>();

    let items = paged_rows
        .into_iter()
        .map(|(form, part_type)| NewPartFormListItem {
            id: form.id,
            part_number: form.part_number,
            part_type_name: part_type.map(|item| item.part_type_name).unwrap_or_else(|| "Unknown".to_string()),
            work_order_number: form.work_order_number,
            status: if form.status.eq_ignore_ascii_case("denied") { "rejected".to_string() } else { form.status },
            created_at: crate::utils::time::to_utc7_string(form.created_at),
        })
        .collect::<Vec<_>>();

    (NewPartFormListResponse { items, summary }, total_records)
}

/// Pure mapping logic for new part form detail response.
pub fn map_detail_response(
    form: crate::entities::new_part_forms::Model,
    part_type_name: String,
    photo_urls: Vec<String>,
    rejection_reason: Option<String>,
) -> NewPartFormDetailResponse {
    NewPartFormDetailResponse {
        id: form.id,
        part_number: form.part_number,
        part_type_name,
        model_code: form.model_code,
        serial_number: form.serial_number,
        work_order_id: form.work_order_id,
        work_order_number: form.work_order_number,
        description: form.description,
        status: if form.status.eq_ignore_ascii_case("denied") { "rejected".to_string() } else { form.status },
        denial_reason: rejection_reason,
        photo_urls,
        created_at: crate::utils::time::to_utc7_string(form.created_at),
        updated_at: crate::utils::time::to_utc7_string(form.updated_at),
    }
}
