use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions};
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use crate::model::responses::inventory::product_list_item::ProductListItem;
use crate::model::responses::pagination::PaginationResponse;

pub struct ProductEntry {
    pub product: prod::Model,
    pub model: product_models::Model,
    pub parts: Vec<PartInProduct>,
}

pub struct PartInProduct {
    pub part: parts::Model,
    pub catalog: part_catalog::Model,
    pub condition: part_conditions::Model,
    pub technician_id: Option<uuid::Uuid>,
}

fn can_user_see(role_name: &str, user_id: uuid::Uuid, e: &ProductEntry) -> bool {
    match role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => e.parts.iter().any(|p| p.technician_id == Some(user_id)),
        "customer" => e.product.customer_id == user_id,
        _ => false,
    }
}

pub fn list_products(
    entries: &[ProductEntry],
    role_name: &str,
    user_id: uuid::Uuid,
    query: &ListProductsQuery,
) -> (Vec<ProductListItem>, PaginationResponse) {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let mut filtered: Vec<&ProductEntry> = entries
        .iter()
        .filter(|e| can_user_see(role_name, user_id, e))
        .filter(|e| {
            if let Some(ref mc) = query.model_code { e.product.product_model_code == *mc } else { true }
        })
        .filter(|e| {
            if let Some(ref s) = query.search {
                let s = s.to_lowercase();
                e.product.product_name.to_lowercase().contains(&s) || e.product.serial_number.to_lowercase().contains(&s)
            } else { true }
        })
        .collect();

    let total = filtered.len() as u64;

    let sort_by = query.sort_by.as_deref().unwrap_or("created_at");
    let asc = query.sort_order.as_deref().unwrap_or("asc") == "asc";
    filtered.sort_by(|a, b| {
        let cmp = match sort_by {
            "product_name" => a.product.product_name.cmp(&b.product.product_name),
            "serial_number" => a.product.serial_number.cmp(&b.product.serial_number),
            "model_code" => a.product.product_model_code.cmp(&b.product.product_model_code),
            _ => a.product.created_at.cmp(&b.product.created_at),
        };
        if asc { cmp } else { cmp.reverse() }
    });

    let offset = ((page - 1) * limit) as usize;
    let items: Vec<ProductListItem> = filtered
        .into_iter()
        .skip(offset)
        .take(limit as usize)
        .map(|e| ProductListItem {
            product_id: e.product.id,
            product_name: e.product.product_name.clone(),
            model_code: e.product.product_model_code.clone(),
            serial_number: e.product.serial_number.clone(),
            part_count: e.parts.len() as i64,
            created_at: e.product.created_at.to_rfc3339(),
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
    fn t() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::parse_from_str("2024-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap())
    }

    fn make_entry(id: &str, mc: &str, sn: &str, cust: &str) -> ProductEntry {
        ProductEntry {
            product: prod::Model { id: u(id), product_model_code: mc.into(), customer_id: u(cust), product_name: format!("Product {mc}"), serial_number: sn.into(), created_at: t(), updated_at: t(), deleted_at: None },
            model: product_models::Model { model_code: mc.into(), model_name: format!("Model {mc}"), description: None, created_at: t(), updated_at: t(), deleted_at: None },
            parts: vec![],
        }
    }

    fn q() -> ListProductsQuery { ListProductsQuery { model_code: None, search: None, page: None, limit: None, sort_by: None, sort_order: None } }

    #[test]
    fn test_admin_sees_all() {
        let admin = u("a0000000-0000-0000-0000-000000000000");
        let entries = vec![
            make_entry("10000000-0000-0000-0000-000000000001", "M-A", "S-1", "c0000000-0000-0000-0000-000000000001"),
            make_entry("20000000-0000-0000-0000-000000000002", "M-B", "S-2", "c0000000-0000-0000-0000-000000000002"),
        ];
        let (items, _) = list_products(&entries, "Admin", admin, &q());
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_customer_sees_only_own() {
        let a = u("c0000000-0000-0000-0000-000000000001");
        let entries = vec![
            make_entry("10000000-0000-0000-0000-000000000001", "M-A", "S-1", "c0000000-0000-0000-0000-000000000001"),
            make_entry("20000000-0000-0000-0000-000000000002", "M-B", "S-2", "c0000000-0000-0000-0000-000000000002"),
        ];
        let (items, _) = list_products(&entries, "Customer", a, &q());
        assert_eq!(items.len(), 1);
    }
}
