use crate::entities::{parts, part_catalog, part_conditions, products};
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::part_list_item::PartListItem;
use crate::model::responses::pagination::PaginationResponse;

/// Joined data for listing parts — the handler assembles this from entity queries.
pub struct PartEntry {
    pub part: parts::Model,
    pub catalog: part_catalog::Model,
    pub condition: part_conditions::Model,
    pub product: Option<products::Model>,
    pub status: String,
    pub denial_reason: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub technician_id: Option<uuid::Uuid>,
}

fn can_user_see(role_name: &str, user_id: uuid::Uuid, e: &PartEntry) -> bool {
    match role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => e.technician_id == Some(user_id),
        "customer" => e.status == "approved" && e.customer_id == Some(user_id),
        _ => false,
    }
}

pub fn list_parts(
    entries: &[PartEntry],
    role_name: &str,
    user_id: uuid::Uuid,
    query: &ListPartsQuery,
) -> (Vec<PartListItem>, PaginationResponse) {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let mut filtered: Vec<&PartEntry> = entries
        .iter()
        .filter(|e| can_user_see(role_name, user_id, e))
        .filter(|e| {
            if let Some(ref mc) = query.model_code {
                e.catalog.part_types_id.to_string() == *mc
            } else { true }
        })
        .filter(|e| {
            if let Some(pt) = query.part_type_id { e.catalog.part_types_id == pt } else { true }
        })
        .filter(|e| {
            if let Some(ref s) = query.approval_status {
                let s = s.to_lowercase();
                if !["pending", "approved", "denied"].contains(&s.as_str()) { return false; }
                e.status == s
            } else { true }
        })
        .filter(|e| {
            if let Some(ref search) = query.search {
                let s = search.to_lowercase();
                e.catalog.part_number.to_lowercase().contains(&s)
                    || e.part.serial_number.to_lowercase().contains(&s)
            } else { true }
        })
        .collect();

    let total = filtered.len() as u64;

    // Sorting
    let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
    let asc = query.sort_order.as_deref().unwrap_or("asc") == "asc";
    filtered.sort_by(|a, b| {
        let cmp = match sort_by {
            "part_number" => a.catalog.part_number.cmp(&b.catalog.part_number),
            "serial_number" => a.part.serial_number.cmp(&b.part.serial_number),
            _ => a.part.created_at.cmp(&b.part.created_at),
        };
        if asc { cmp } else { cmp.reverse() }
    });

    let offset = ((page - 1) * limit) as usize;
    let items: Vec<PartListItem> = filtered
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .map(|e| PartListItem {
            part_id: e.part.id,
            part_number: e.catalog.part_number.clone(),
            part_type_name: e.catalog.part_types_id.to_string(),
            serial_number: e.part.serial_number.clone(),
            condition_name: e.condition.name.clone(),
            product_name: e.product.as_ref().map(|p| p.product_name.clone()),
            approval_status: e.status.clone(),
            created_at: e.part.created_at.to_rfc3339(),
        })
        .collect();

    let total_pages = (total as f64 / limit as f64).ceil() as u64;

    (items, PaginationResponse { current_page: page, limit, total_pages, total_records: total, has_next: page < total_pages })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::TimeZone;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
    fn dt(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").unwrap())
    }
    fn t() -> chrono::DateTime<chrono::Utc> { dt("2024-01-01T00:00:00") }

    fn entry(id: &str, pn: &str, sn: &str, status: &str, tech: Option<&str>, cust: Option<&str>) -> PartEntry {
        PartEntry {
            part: parts::Model {
                id: u(id), part_catalog_id: u("00000000-0000-0000-0000-000000000000"),
                product_id: None, serial_number: sn.to_string(), part_condition_id: 1,
                manufactured_date: t(), installation_date: None, removal_date: None, scrapped_date: None,
                created_at: t(), updated_at: t(), deleted_at: None,
            },
            catalog: part_catalog::Model {
                id: u("00000000-0000-0000-0000-000000000000"), part_number: pn.to_string(),
                part_types_id: 1, mfg_number: "MFG".to_string(), description: None, part_mfg_status: 1,
                created_at: t(), updated_at: t(), deleted_at: None,
            },
            condition: part_conditions::Model { id: 1, name: "New".to_string() },
            product: None, status: status.to_string(), denial_reason: None,
            customer_id: cust.map(u), technician_id: tech.map(u),
        }
    }

    fn q() -> ListPartsQuery {
        ListPartsQuery { model_code: None, part_type_id: None, approval_status: None,
            search: None, page: None, limit: None, sort_by: None, sort_order: None }
    }

    #[test]
    fn test_admin_sees_all() {
        let admin = u("a0000000-0000-0000-0000-000000000000");
        let entries = vec![entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "pending", Some("b0000000-0000-0000-0000-000000000001"), None)];
        let (items, _) = list_parts(&entries, "Admin", admin, &q());
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_technician_sees_only_own() {
        let tech_a = u("b0000000-0000-0000-0000-000000000001");
        let entries = vec![
            entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "pending", Some("b0000000-0000-0000-0000-000000000001"), None),
            entry("20000000-0000-0000-0000-000000000000", "PN-B", "SN-2", "approved", Some("d0000000-0000-0000-0000-000000000000"), None),
        ];
        let (items, _) = list_parts(&entries, "Technician", tech_a, &q());
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_customer_sees_only_approved_own() {
        let cust = u("c0000000-0000-0000-0000-000000000001");
        let entries = vec![
            entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "approved", None, Some("c0000000-0000-0000-0000-000000000001")),
            entry("20000000-0000-0000-0000-000000000000", "PN-B", "SN-2", "pending", None, Some("c0000000-0000-0000-0000-000000000001")),
        ];
        let (items, _) = list_parts(&entries, "Customer", cust, &q());
        assert_eq!(items.len(), 1);
    }
}
