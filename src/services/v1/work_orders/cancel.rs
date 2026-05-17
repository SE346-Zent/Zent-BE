use chrono::{Duration, Utc};
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_orders, work_order_state_history},
};

#[derive(Debug)]
pub struct CancelWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

/// Pure logic: decide whether a customer can cancel their work order.
///
/// Rule: the customer can cancel only if there are more than 24 hours
/// remaining before the appointment. If within 24 hours, the cancel is refused.
pub fn decide_cancel_work_order(
    work_order: work_orders::Model,
    closed_status_id: i32,
    customer_id: Uuid,
    cancel_window_hours: i64,
) -> Result<CancelWorkOrderEffect, AppError> {
    // Only the owner of the work order can cancel it
    if work_order.customer_id != customer_id {
        return Err(AppError::Forbidden("You can only cancel your own work orders".to_string()));
    }

    let now = Utc::now();
    let cutoff = work_order.appointment - Duration::hours(cancel_window_hours);

    if now >= cutoff {
        return Err(AppError::BadRequest(format!(
            "Cannot cancel within {} hours of the appointment. Please contact support for assistance.",
            cancel_window_hours
        )));
    }

    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(closed_status_id);
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(closed_status_id),
        changed_by_id: Set(customer_id),
        changed_at: Set(now),
    };

    Ok(CancelWorkOrderEffect { work_order: active_wo, state_history })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::Set;

    fn dummy_work_order(customer_id: Uuid, status_id: i32) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: status_id,
            customer_id,
            product_id: Uuid::new_v4(),
            reference_ticket_id: None,
            work_order_symptom_id: 1,
            description: "".to_string(),
            first_name: "".to_string(),
            last_name: "".to_string(),
            email: None,
            phone_number: None,
            country: "".to_string(),
            province: "".to_string(),
            city: "".to_string(),
            address: "".to_string(),
            building: None,
            appointment: Utc::now() + chrono::Duration::hours(48), // far future — passes 24h check
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            customer_complaint: None,
            customer_complaint_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_success() {
        let customer_id = Uuid::new_v4();
        let closed_status_id = 4;
        let wo = dummy_work_order(customer_id, 1);

        let result = decide_cancel_work_order(wo, closed_status_id, customer_id, 24);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.work_order_status_id, Set(closed_status_id));
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_wrong_customer() {
        let customer_id = Uuid::new_v4();
        let wrong_customer_id = Uuid::new_v4();
        let wo = dummy_work_order(customer_id, 1);

        let result = decide_cancel_work_order(wo, 4, wrong_customer_id, 24);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Forbidden(_) => {}
            other => panic!("Expected Forbidden, got {:?}", other),
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_too_close_to_appointment() {
        let customer_id = Uuid::new_v4();
        let mut wo = dummy_work_order(customer_id, 1);
        // Appointment is only 1 hour away — within the 24h window
        wo.appointment = Utc::now() + chrono::Duration::hours(1);

        let result = decide_cancel_work_order(wo, 4, customer_id, 24);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(_) => {}
            other => panic!("Expected BadRequest, got {:?}", other),
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_just_before_cutoff() {
        let customer_id = Uuid::new_v4();
        let mut wo = dummy_work_order(customer_id, 1);
        // Appointment is 25 hours away — just outside the 24h window
        wo.appointment = Utc::now() + chrono::Duration::hours(25);

        let result = decide_cancel_work_order(wo, 4, customer_id, 24);
        assert!(result.is_ok());
    }
}
