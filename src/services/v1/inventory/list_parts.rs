use crate::entities::{parts, part_catalog, part_conditions, products};
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::part_list_item::PartListItem;
use crate::model::responses::pagination::PaginationResponse;

/// Represents a single part entry as assembled from multiple database tables for listing purposes.
pub struct PartEntry {
    /// The core part record containing serial and status data.
    pub part_record: parts::Model,
    /// The associated catalog definition (e.g., part number, type).
    pub catalog_definition: part_catalog::Model,
    /// The current physical condition of the part.
    pub physical_condition: part_conditions::Model,
    /// The product this part is currently installed in, if any.
    pub installed_product: Option<products::Model>,
    /// The current approval status string (e.g., 'pending', 'approved').
    pub approval_status: String,
    /// Optional reason if the part addition was denied.
    pub denial_reason: Option<String>,
    /// The ID of the customer who owns the product this part is in.
    pub customer_id: Option<uuid::Uuid>,
    /// The ID of the technician who registered this part.
    pub technician_id: Option<uuid::Uuid>,
}

/// Determine if a user with a specific role is permitted to see a particular part entry.
///
/// Visibility rules:
/// - Admins and Managers can see all parts.
/// - Technicians can see parts they registered.
/// - Customers can see approved parts belonging to their products.
fn can_user_see_part(
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    part_entry: &PartEntry,
) -> bool {
    match requesting_role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => part_entry.technician_id == Some(requesting_user_id),
        "customer" => {
            part_entry.approval_status == "approved" && part_entry.customer_id == Some(requesting_user_id)
        }
        _ => false,
    }
}

/// Assemble a paginated list of part items based on user role and query filters.
///
/// This function performs in-memory filtering (including visibility checks), 
/// sorting, and pagination on a pre-fetched set of `PartEntry` data.
///
/// # Arguments
/// * `assembled_entries` - The list of part entries joined with related table data.
/// * `requesting_role_name` - The role of the user requesting the list.
/// * `requesting_user_id` - The unique identifier of the requesting user.
/// * `list_query` - The query parameters for filtering, sorting, and pagination.
///
/// # Returns
/// A tuple containing the list of `PartListItem` and the `PaginationResponse` metadata.
pub fn list_parts(
    assembled_entries: &[PartEntry],
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    list_query: &ListPartsQuery,
) -> (Vec<PartListItem>, PaginationResponse) {
    let current_page = list_query.page.unwrap_or(1).max(1);
    let page_limit = list_query.limit.unwrap_or(20).clamp(1, 100);

    let mut filtered_entries: Vec<&PartEntry> = assembled_entries
        .iter()
        .filter(|entry| can_user_see_part(requesting_role_name, requesting_user_id, entry))
        .filter(|entry| {
            if let Some(ref model_code) = list_query.model_code {
                entry.catalog_definition.part_types_id.to_string() == *model_code
            } else { true }
        })
        .filter(|entry| {
            if let Some(pt_id) = list_query.part_type_id { entry.catalog_definition.part_types_id == pt_id } else { true }
        })
        .filter(|entry| {
            if let Some(ref status) = list_query.approval_status {
                let status_lower = status.to_lowercase();
                if !["pending", "approved", "denied"].contains(&status_lower.as_str()) { return false; }
                entry.approval_status == status_lower
            } else { true }
        })
        .filter(|entry| {
            if let Some(ref search_term) = list_query.search {
                let term = search_term.to_lowercase();
                entry.catalog_definition.part_number.to_lowercase().contains(&term)
                    || entry.part_record.serial_number.to_lowercase().contains(&term)
            } else { true }
        })
        .collect();

    let total_records = filtered_entries.len() as u64;

    // Sorting
    let sort_field = list_query.sort_by.as_deref().unwrap_or("created_at");
    let is_ascending = list_query.sort_order.as_deref().unwrap_or("asc") == "asc";
    filtered_entries.sort_by(|a, b| {
        let cmp = match sort_field {
            "part_number" => a.catalog_definition.part_number.cmp(&b.catalog_definition.part_number),
            "serial_number" => a.part_record.serial_number.cmp(&b.part_record.serial_number),
            _ => a.part_record.created_at.cmp(&b.part_record.created_at),
        };
        if is_ascending { cmp } else { cmp.reverse() }
    });

    let page_offset = ((current_page - 1) * page_limit) as usize;
    let paginated_items: Vec<PartListItem> = filtered_entries
        .into_iter()
        .skip(page_offset)
        .take(page_limit as usize)
        .map(|entry| PartListItem {
            part_id: entry.part_record.id,
            part_number: entry.catalog_definition.part_number.clone(),
            part_type_name: entry.catalog_definition.part_types_id.to_string(),
            serial_number: entry.part_record.serial_number.clone(),
            condition_name: entry.physical_condition.name.clone(),
            product_name: entry.installed_product.as_ref().map(|p| p.product_name.clone()),
            approval_status: entry.approval_status.clone(),
            created_at: entry.part_record.created_at.to_rfc3339(),
        })
        .collect();

    let total_pages = (total_records as f64 / page_limit as f64).ceil() as u64;

    (paginated_items, PaginationResponse { 
        current_page, 
        limit: page_limit, 
        total_pages, 
        total_records, 
        has_next: current_page < total_pages 
    })
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
            part_record: parts::Model {
                id: u(id), part_catalog_id: u("00000000-0000-0000-0000-000000000000"),
                product_id: None, serial_number: sn.to_string(), part_condition_id: 1,
                manufactured_date: t(), installation_date: None, removal_date: None, scrapped_date: None,
                created_at: t(), updated_at: t(), deleted_at: None,
            },
            catalog_definition: part_catalog::Model {
                id: u("00000000-0000-0000-0000-000000000000"), part_number: pn.to_string(),
                part_types_id: 1, mfg_number: "MFG".to_string(), description: None, part_mfg_status: 1,
                created_at: t(), updated_at: t(), deleted_at: None,
            },
            physical_condition: part_conditions::Model { id: 1, name: "New".to_string() },
            installed_product: None, 
            approval_status: status.to_string(), 
            denial_reason: None,
            customer_id: cust.map(u), 
            technician_id: tech.map(u),
        }
    }

    fn q() -> ListPartsQuery {
        ListPartsQuery { model_code: None, part_type_id: None, approval_status: None,
            search: None, page: None, limit: None, sort_by: None, sort_order: None }
    }

    #[test]
    fn test_admin_sees_all() {
        let admin_user_id = u("a0000000-0000-0000-0000-000000000000");
        let assembled_entries = vec![entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "pending", Some("b0000000-0000-0000-0000-000000000001"), None)];
        let (paginated_items, _) = list_parts(&assembled_entries, "Admin", admin_user_id, &q());
        assert_eq!(paginated_items.len(), 1);
    }

    #[test]
    fn test_technician_sees_only_own() {
        let tech_user_id = u("b0000000-0000-0000-0000-000000000001");
        let assembled_entries = vec![
            entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "pending", Some("b0000000-0000-0000-0000-000000000001"), None),
            entry("20000000-0000-0000-0000-000000000000", "PN-B", "SN-2", "approved", Some("d0000000-0000-0000-0000-000000000000"), None),
        ];
        let (paginated_items, _) = list_parts(&assembled_entries, "Technician", tech_user_id, &q());
        assert_eq!(paginated_items.len(), 1);
    }

    #[test]
    fn test_customer_sees_only_approved_own() {
        let customer_user_id = u("c0000000-0000-0000-0000-000000000001");
        let assembled_entries = vec![
            entry("10000000-0000-0000-0000-000000000000", "PN-A", "SN-1", "approved", None, Some("c0000000-0000-0000-0000-000000000001")),
            entry("20000000-0000-0000-0000-000000000000", "PN-B", "SN-2", "pending", None, Some("c0000000-0000-0000-0000-000000000001")),
        ];
        let (paginated_items, _) = list_parts(&assembled_entries, "Customer", customer_user_id, &q());
        assert_eq!(paginated_items.len(), 1);
    }
}
