use crate::entities::warranties;
use crate::model::responses::inventory::warranty_check_response::WarrantyCheckResponse;
use uuid::Uuid;

/// Determine the warranty status of a product by matching it against local warranty records.
///
/// If a matching warranty is found, its status is computed based on current time.
pub fn determine_warranty_status(
    product_id: Uuid,
    serial_number: &str,
    product_name: &str,
    existing_warranty: Option<warranties::Model>,
    current_time: chrono::DateTime<chrono::Utc>,
) -> WarrantyCheckResponse {
    match existing_warranty {
        Some(w) => {
            let status = if current_time > w.end_date {
                "expired".to_string()
            } else {
                w.warranty_status.clone()
            };
            WarrantyCheckResponse {
                product_id,
                serial_number: serial_number.to_string(),
                product_name: product_name.to_string(),
                warranty_status: status,
                start_date: Some(w.start_date.to_rfc3339()),
                end_date: Some(w.end_date.to_rfc3339()),
            }
        }
        None => WarrantyCheckResponse {
            product_id,
            serial_number: serial_number.to_string(),
            product_name: product_name.to_string(),
            warranty_status: "none".to_string(),
            start_date: None,
            end_date: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }

    #[test]
    fn test_active_warranty() {
        let now = chrono::Utc::now();
        let w = warranties::Model {
            id: u("00000000-0000-0000-0000-000000000001"),
            customer_id: u("00000000-0000-0000-0000-000000000002"),
            product_id: u("00000000-0000-0000-0000-000000000003"),
            start_date: now - chrono::Duration::days(10),
            end_date: now + chrono::Duration::days(10),
            warranty_status: "active".to_string(),
            warranty_status_id: Some(1),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        let res = determine_warranty_status(w.product_id, "SN-123", "iPhone 15", Some(w), now);
        assert_eq!(res.warranty_status, "active");
        assert!(res.start_date.is_some());
        assert!(res.end_date.is_some());
    }

    #[test]
    fn test_expired_warranty() {
        let now = chrono::Utc::now();
        let w = warranties::Model {
            id: u("00000000-0000-0000-0000-000000000001"),
            customer_id: u("00000000-0000-0000-0000-000000000002"),
            product_id: u("00000000-0000-0000-0000-000000000003"),
            start_date: now - chrono::Duration::days(20),
            end_date: now - chrono::Duration::days(10),
            warranty_status: "active".to_string(),
            warranty_status_id: Some(1),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        let res = determine_warranty_status(w.product_id, "SN-123", "iPhone 15", Some(w), now);
        assert_eq!(res.warranty_status, "expired");
    }

    #[test]
    fn test_no_warranty() {
        let now = chrono::Utc::now();
        let prod_id = u("00000000-0000-0000-0000-000000000003");
        let res = determine_warranty_status(prod_id, "SN-123", "iPhone 15", None, now);
        assert_eq!(res.warranty_status, "none");
        assert!(res.start_date.is_none());
        assert!(res.end_date.is_none());
    }
}
