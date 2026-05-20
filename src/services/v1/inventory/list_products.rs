use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions};
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use crate::model::responses::inventory::product_list_item::ProductListItem;
use crate::model::responses::pagination::PaginationResponse;

/// Represents a single product entry as assembled from multiple database tables for listing purposes.
pub struct ProductEntry {
    /// The core product record containing serial and assignment data.
    pub product_record: prod::Model,
    /// The product model definition (e.g., model name, description).
    pub model_definition: product_models::Model,
    /// A list of parts currently installed in this product.
    pub installed_parts: Vec<PartInProduct>,
}

/// Represents a part associated with a product, including its catalog and condition data.
pub struct PartInProduct {
    /// The core part record.
    pub part_record: parts::Model,
    /// The associated catalog definition.
    pub catalog_definition: part_catalog::Model,
    /// The current physical condition of the part.
    pub physical_condition: part_conditions::Model,
    /// The ID of the technician who registered this part.
    pub registering_technician_id: Option<uuid::Uuid>,
}

/// Determine if a user with a specific role is permitted to see a particular product entry.
///
/// Visibility rules:
/// - Admins and Managers can see all products.
/// - Technicians can see products if they registered at least one part currently in it.
/// - Customers can see products they registered.
fn can_user_see_product(
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    product_entry: &ProductEntry,
) -> bool {
    match requesting_role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => product_entry.installed_parts.iter().any(|part| part.registering_technician_id == Some(requesting_user_id)),
        "customer" => product_entry.product_record.customer_id == requesting_user_id,
        _ => false,
    }
}

/// Assemble a paginated list of product items based on user role and query filters.
///
/// This function performs in-memory filtering (including visibility checks), 
/// sorting, and pagination on a pre-fetched set of `ProductEntry` data.
///
/// # Arguments
/// * `assembled_entries` - The list of product entries joined with related table data.
/// * `requesting_role_name` - The role of the user requesting the list.
/// * `requesting_user_id` - The unique identifier of the requesting user.
/// * `list_query` - The query parameters for filtering, sorting, and pagination.
///
/// # Returns
/// A tuple containing the list of `ProductListItem` and the `PaginationResponse` metadata.
pub fn list_products(
    assembled_entries: &[ProductEntry],
    requesting_role_name: &str,
    requesting_user_id: uuid::Uuid,
    list_query: &ListProductsQuery,
) -> (Vec<ProductListItem>, PaginationResponse) {
    let current_page = list_query.page.unwrap_or(1).max(1);
    let page_limit = list_query.limit.unwrap_or(20).clamp(1, 100);

    let mut filtered_entries: Vec<&ProductEntry> = assembled_entries
        .iter()
        .filter(|entry| can_user_see_product(requesting_role_name, requesting_user_id, entry))
        .filter(|entry| {
            if let Some(ref model_code) = list_query.model_code { 
                entry.product_record.product_model_code == *model_code 
            } else { true }
        })
        .filter(|entry| {
            if let Some(ref search_term) = list_query.search {
                let term = search_term.to_lowercase();
                entry.product_record.product_name.to_lowercase().contains(&term) 
                    || entry.product_record.serial_number.to_lowercase().contains(&term)
            } else { true }
        })
        .collect();

    let total_records = filtered_entries.len() as u64;

    // Sorting
    let sort_field = list_query.sort_by.as_deref().unwrap_or("created_at");
    let is_ascending = list_query.sort_order.as_deref().unwrap_or("asc") == "asc";
    filtered_entries.sort_by(|a, b| {
        let cmp = match sort_field {
            "product_name" => a.product_record.product_name.cmp(&b.product_record.product_name),
            "serial_number" => a.product_record.serial_number.cmp(&b.product_record.serial_number),
            "model_code" => a.product_record.product_model_code.cmp(&b.product_record.product_model_code),
            _ => a.product_record.created_at.cmp(&b.product_record.created_at),
        };
        if is_ascending { cmp } else { cmp.reverse() }
    });

    let page_offset = ((current_page - 1) * page_limit) as usize;
    let paginated_items: Vec<ProductListItem> = filtered_entries
        .into_iter()
        .skip(page_offset)
        .take(page_limit as usize)
        .map(|entry| ProductListItem {
            product_id: entry.product_record.id,
            product_name: entry.product_record.product_name.clone(),
            model_code: entry.product_record.product_model_code.clone(),
            serial_number: entry.product_record.serial_number.clone(),
            part_count: entry.installed_parts.len() as i64,
            created_at: entry.product_record.created_at.to_rfc3339(),
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
    fn t() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::parse_from_str("2024-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap())
    }

    fn make_entry(id: &str, mc: &str, sn: &str, cust: &str) -> ProductEntry {
        ProductEntry {
            product_record: prod::Model { id: u(id), product_model_code: mc.into(), customer_id: u(cust), product_name: format!("Product {mc}"), serial_number: sn.into(), created_at: t(), updated_at: t(), deleted_at: None },
            model_definition: product_models::Model { model_code: mc.into(), model_name: format!("Model {mc}"), description: None, created_at: t(), updated_at: t(), deleted_at: None },
            installed_parts: vec![],
        }
    }

    fn q() -> ListProductsQuery { ListProductsQuery { model_code: None, search: None, page: None, limit: None, sort_by: None, sort_order: None } }

    #[test]
    fn test_admin_sees_all() {
        let admin_user_id = u("a0000000-0000-0000-0000-000000000000");
        let assembled_entries = vec![
            make_entry("10000000-0000-0000-0000-000000000001", "M-A", "S-1", "c0000000-0000-0000-0000-000000000001"),
            make_entry("20000000-0000-0000-0000-000000000002", "M-B", "S-2", "c0000000-0000-0000-0000-000000000002"),
        ];
        let (paginated_items, _) = list_products(&assembled_entries, "Admin", admin_user_id, &q());
        assert_eq!(paginated_items.len(), 2);
    }

    #[test]
    fn test_customer_sees_only_own() {
        let customer_user_id = u("c0000000-0000-0000-0000-000000000001");
        let assembled_entries = vec![
            make_entry("10000000-0000-0000-0000-000000000001", "M-A", "S-1", "c0000000-0000-0000-0000-000000000001"),
            make_entry("20000000-0000-0000-0000-000000000002", "M-B", "S-2", "c0000000-0000-0000-0000-000000000002"),
        ];
        let (paginated_items, _) = list_products(&assembled_entries, "Customer", customer_user_id, &q());
        assert_eq!(paginated_items.len(), 1);
    }
}
