use chrono::{Duration, Utc};
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{warranties, work_orders},
    model::requests::work_orders::edit_request::EditWorkOrderRequest,
};

/// Snapshot of the work order fields the customer is allowed to edit.
///
/// The `product_id_changed` flag is `true` whenever the customer supplied a new
/// `product_id` value that differs from the one currently stored on the work order.
#[derive(Debug, Clone)]
pub struct EditWorkOrderContext {
    /// The new `product_id` requested by the customer, if any.
    pub new_product_id: Option<Uuid>,
    /// Whether the requested `product_id` differs from the current one.
    pub product_id_changed: bool,
    /// Warranty record (if any) for the new product. Only populated when the
    /// customer is changing the product.
    pub new_product_warranty: Option<warranties::Model>,
    /// The current time, captured by the caller for deterministic tests.
    pub now: chrono::DateTime<Utc>,
}

/// The side-effect of a successful work order edit.
#[derive(Debug)]
pub struct EditWorkOrderEffect {
    /// The updated work order model (with only the changed fields populated).
    pub work_order_model: work_orders::ActiveModel,
    /// Whether the customer requested a new product.
    pub product_id_changed: bool,
    /// The technician assigned to the work order before the edit (if any).
    pub old_technician_id: Option<Uuid>,
}

/// Pure logic: validate and produce the updated work order model for a customer edit.
///
/// # Rules
/// - Only the work order owner is allowed to edit it.
/// - Editing is only permitted while the work order is `Pending` or `Assigned`. Once a
///   technician has started the work, the customer can no longer modify it.
/// - Editing is only permitted until `edit_window_hours` before the scheduled
///   appointment. After that window, FE should hide the edit form.
/// - If the customer is changing the `product_id`, the new product must:
///   * belong to the same customer (enforced by the caller via the Zeus client), and
///   * be covered by an **active** warranty whose `end_date` is in the future.
///   If the warranty is missing, expired, or voided, the request is rejected with an
///   `AppError::WarrantyError` carrying a user-friendly message that the FE can
///   surface directly to the customer.
pub fn decide_edit_work_order(
    work_order: work_orders::Model,
    requesting_customer_id: Uuid,
    pending_status_id: i32,
    assigned_status_id: i32,
    edit_window_hours: i64,
    payload: EditWorkOrderRequest,
    ctx: EditWorkOrderContext,
) -> Result<EditWorkOrderEffect, AppError> {
    // 1. Ownership check
    if work_order.customer_id != requesting_customer_id {
        return Err(AppError::Forbidden("NOT_WORK_ORDER_OWNER".to_string()));
    }

    // 2. Status check — editing is only allowed while the work is not yet in progress
    if work_order.work_order_status_id != pending_status_id
        && work_order.work_order_status_id != assigned_status_id
    {
        return Err(AppError::BadRequest("WORK_ORDER_NOT_EDITABLE".to_string()));
    }

    // 3. Edit window check
    let now = ctx.now;
    let edit_cutoff = work_order.appointment - Duration::hours(edit_window_hours);
    if now >= edit_cutoff {
        return Err(AppError::BadRequest("EDIT_WINDOW_EXPIRED".to_string()));
    }

    // 4. Warranty validation when the product is changing
    if ctx.product_id_changed {
        match ctx.new_product_warranty {
            Some(w) => {
                let now = ctx.now;
                if now > w.end_date {
                    return Err(AppError::WarrantyError("PRODUCT_WARRANTY_EXPIRED".to_string()));
                }
                let status_lower = w.warranty_status.to_lowercase();
                if status_lower != "active" {
                    return Err(AppError::WarrantyError("PRODUCT_WARRANTY_NOT_ACTIVE".to_string()));
                }
            }
            None => {
                return Err(AppError::WarrantyError("PRODUCT_NOT_UNDER_WARRANTY".to_string()));
            }
        }
    }

    // 5. Parse and validate appointment if changing
    let parsed_appointment = if let Some(ref appointment_str) = payload.appointment {
        let parsed = chrono::DateTime::parse_from_rfc3339(appointment_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(appointment_str, "%Y-%m-%dT%H:%M:%S")
                    .map(|nd| nd.and_utc())
            })
            .map_err(|_| AppError::BadRequest("INVALID_APPOINTMENT_FORMAT".to_string()))?;

        if parsed <= now {
            return Err(AppError::BadRequest("APPOINTMENT_MUST_BE_IN_FUTURE".to_string()));
        }
        Some(parsed)
    } else {
        None
    };

    // 6. Apply changes — only update fields that were supplied
    let old_technician_id = work_order.technician_id;
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();

    if let Some(new_product) = ctx.new_product_id {
        active_wo.product_id = Set(new_product);
    }
    if let Some(new_ward) = payload.ward {
        active_wo.ward = Set(new_ward);
    }
    if let Some(new_address) = payload.address {
        active_wo.address = Set(new_address);
    }
    if let Some(new_building) = payload.building {
        active_wo.building = Set(Some(new_building));
    }
    if let Some(new_appointment) = parsed_appointment {
        active_wo.appointment = Set(new_appointment);
    }

    active_wo.updated_at = Set(now);

    Ok(EditWorkOrderEffect {
        work_order_model: active_wo,
        product_id_changed: ctx.product_id_changed,
        old_technician_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dummy_work_order(customer_id: Uuid, status_id: i32) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: status_id,
            customer_id,
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "Original".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: None,
            phone_number: None,
            country: "VN".to_string(),
            province: "HCM".to_string(),
            ward: "Ward 1".to_string(),
            address: "123 Street".to_string(),
            building: Some("B1".to_string()),
            appointment: Utc.with_ymd_and_hms(2099, 1, 1, 3, 0, 0).unwrap(),
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "WO-1".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    fn active_warranty(product_id: Uuid, customer_id: Uuid) -> warranties::Model {
        let now = Utc::now();
        warranties::Model {
            id: Uuid::new_v4(),
            customer_id,
            product_id,
            start_date: now - Duration::days(30),
            end_date: now + Duration::days(30),
            warranty_status: "Active".to_string(),
            warranty_status_id: Some(1),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    fn empty_ctx(wo: &work_orders::Model, new_product: Option<Uuid>, warranty: Option<warranties::Model>) -> EditWorkOrderContext {
        EditWorkOrderContext {
            new_product_id: new_product,
            product_id_changed: new_product.map(|p| p != wo.product_id).unwrap_or(false),
            new_product_warranty: warranty,
            now: Utc::now(),
        }
    }

    #[test]
    fn test_edit_ward_only() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: Some("Ward 5".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        assert!(result.is_ok());
        let eff = result.unwrap();
        assert_eq!(eff.work_order_model.ward, Set("Ward 5".to_string()));
        assert!(!eff.product_id_changed);
        assert!(eff.old_technician_id.is_none());
    }

    #[test]
    fn test_edit_wrong_owner_is_forbidden() {
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        let wo = dummy_work_order(owner, 1);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: Some("Ward 2".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, other, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::Forbidden(msg)) => assert_eq!(msg, "NOT_WORK_ORDER_OWNER"),
            other => panic!("Expected Forbidden NOT_WORK_ORDER_OWNER, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_in_progress_is_rejected() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 3);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: Some("Ward 2".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::BadRequest(msg)) => assert_eq!(msg, "WORK_ORDER_NOT_EDITABLE"),
            other => panic!("Expected BadRequest WORK_ORDER_NOT_EDITABLE, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_within_window_is_rejected() {
        let customer = Uuid::new_v4();
        let mut wo = dummy_work_order(customer, 1);
        wo.appointment = Utc::now() + Duration::hours(1);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: Some("Ward 2".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::BadRequest(msg)) => assert_eq!(msg, "EDIT_WINDOW_EXPIRED"),
            other => panic!("Expected BadRequest EDIT_WINDOW_EXPIRED, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_product_without_warranty_is_rejected() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let new_product = Uuid::new_v4();
        let ctx = empty_ctx(&wo, Some(new_product), None);
        let payload = EditWorkOrderRequest {
            product_id: Some(new_product),
            ward: None,
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::WarrantyError(msg)) => assert_eq!(msg, "PRODUCT_NOT_UNDER_WARRANTY"),
            other => panic!("Expected WarrantyError PRODUCT_NOT_UNDER_WARRANTY, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_product_with_expired_warranty_is_rejected() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let new_product = Uuid::new_v4();
        let mut w = active_warranty(new_product, customer);
        w.end_date = Utc::now() - Duration::days(1);
        let ctx = empty_ctx(&wo, Some(new_product), Some(w));
        let payload = EditWorkOrderRequest {
            product_id: Some(new_product),
            ward: None,
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::WarrantyError(msg)) => assert_eq!(msg, "PRODUCT_WARRANTY_EXPIRED"),
            other => panic!("Expected WarrantyError PRODUCT_WARRANTY_EXPIRED, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_product_with_voided_warranty_is_rejected() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let new_product = Uuid::new_v4();
        let mut w = active_warranty(new_product, customer);
        w.warranty_status = "Voided".to_string();
        let ctx = empty_ctx(&wo, Some(new_product), Some(w));
        let payload = EditWorkOrderRequest {
            product_id: Some(new_product),
            ward: None,
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::WarrantyError(msg)) => assert_eq!(msg, "PRODUCT_WARRANTY_NOT_ACTIVE"),
            other => panic!("Expected WarrantyError PRODUCT_WARRANTY_NOT_ACTIVE, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_product_with_active_warranty_succeeds() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let new_product = Uuid::new_v4();
        let w = active_warranty(new_product, customer);
        let ctx = empty_ctx(&wo, Some(new_product), Some(w));
        let payload = EditWorkOrderRequest {
            product_id: Some(new_product),
            ward: Some("Ward 9".to_string()),
            address: Some("999 New Street".to_string()),
            building: Some("Tower B".to_string()),
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        assert!(result.is_ok());
        let eff = result.unwrap();
        assert_eq!(eff.work_order_model.product_id, Set(new_product));
        assert_eq!(eff.work_order_model.ward, Set("Ward 9".to_string()));
        assert_eq!(eff.work_order_model.address, Set("999 New Street".to_string()));
        assert_eq!(eff.work_order_model.building, Set(Some("Tower B".to_string())));
        assert!(eff.product_id_changed);
    }

    #[test]
    fn test_edit_same_product_skips_warranty_check() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let ctx = empty_ctx(&wo, Some(wo.product_id), None);
        let payload = EditWorkOrderRequest {
            product_id: Some(wo.product_id),
            ward: Some("Ward 3".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_edit_appointment_must_be_future() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: None,
            address: None,
            building: None,
            appointment: Some("2020-01-01T10:00:00Z".to_string()),
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::BadRequest(msg)) => assert_eq!(msg, "APPOINTMENT_MUST_BE_IN_FUTURE"),
            other => panic!("Expected BadRequest APPOINTMENT_MUST_BE_IN_FUTURE, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_invalid_appointment_format() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: None,
            address: None,
            building: None,
            appointment: Some("not-a-date".to_string()),
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        match result {
            Err(AppError::BadRequest(msg)) => assert_eq!(msg, "INVALID_APPOINTMENT_FORMAT"),
            other => panic!("Expected BadRequest INVALID_APPOINTMENT_FORMAT, got {:?}", other),
        }
    }

    #[test]
    fn test_edit_appointment_valid_future_succeeds() {
        let customer = Uuid::new_v4();
        let wo = dummy_work_order(customer, 1);
        let ctx = empty_ctx(&wo, None, None);
        let future = (Utc::now() + Duration::days(30)).format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: None,
            address: None,
            building: None,
            appointment: Some(future),
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_edit_preserves_old_technician_id() {
        let customer = Uuid::new_v4();
        let mut wo = dummy_work_order(customer, 2);
        let tech_id = Uuid::new_v4();
        wo.technician_id = Some(tech_id);
        let ctx = empty_ctx(&wo, None, None);
        let payload = EditWorkOrderRequest {
            product_id: None,
            ward: Some("Ward 10".to_string()),
            address: None,
            building: None,
            appointment: None,
        };
        let result = decide_edit_work_order(wo, customer, 1, 2, 5, payload, ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().old_technician_id, Some(tech_id));
    }
}
