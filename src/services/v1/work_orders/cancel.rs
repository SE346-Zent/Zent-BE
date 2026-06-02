use chrono::{Duration, Utc};
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{cancel_reasons, work_order_state_history, work_orders},
};

/// Represents the calculated results and side-effects of a successful work order cancellation.

#[derive(Debug)]
pub struct CancelWorkOrderEffect {
    /// The database model for the updated work order (transitioned to 'Closed' status).
    pub work_order_model: work_orders::ActiveModel,
    /// The database model for the state history entry recording the cancellation event.
    pub state_history_model: work_order_state_history::ActiveModel,
    /// The database model for the cancellation reason.
    pub cancel_reason_model: cancel_reasons::ActiveModel,
}

/// Determine if a customer can cancel their work order based on the lead time before the appointment.
///
/// This function enforces a business rule (typically 24 hours) where cancellations
/// are only permitted if requested sufficiently in advance of the scheduled time.
///
/// # Arguments
/// * `work_order` - The database model for the work order to be cancelled.
/// * `target_closed_status_id` - The database ID for the 'Closed' status.
/// * `requesting_customer_id` - The unique ID of the customer requesting the cancellation.
/// * `cancel_window_hours` - The minimum hours required before appointment to allow cancellation.
///
/// # Returns
/// A result containing the `CancelWorkOrderEffect` on success, or an `AppError`.

/// Pure logic: decide whether a customer can cancel their work order.
///
/// Rule: the customer can cancel only if there are more than 24 hours
/// remaining before the appointment. If within 24 hours, the cancel is refused.
pub fn decide_cancel_work_order(
    work_order: work_orders::Model,
    target_closed_status_id: i32,
    requesting_customer_id: Uuid,
    cancel_window_hours: i64,
    reason: String,
    additional_comments: Option<String>,
) -> Result<CancelWorkOrderEffect, AppError> {
    // Only the owner of the work order can cancel it
    if work_order.customer_id != requesting_customer_id {
        tracing::warn!(
            error.message = "NotWorkOrderOwner",
            error.details = "",
            work_order_id = %work_order.id,
            customer_id = %work_order.customer_id,
            requesting_customer_id = %requesting_customer_id,
            message = "You can only cancel your own work orders"
        );
        return Err(AppError::Forbidden(
            "You can only cancel your own work orders".to_string(),
        ));
    }

    let current_timestamp = Utc::now();
    let cancellation_cutoff = work_order.appointment - Duration::hours(cancel_window_hours);

    if current_timestamp >= cancellation_cutoff {
        tracing::warn!(
            error.message = "CancellationWindowPassed",
            error.details = "",
            work_order_id = %work_order.id,
            appointment = %work_order.appointment,
            cancel_window_hours = %cancel_window_hours,
            message = "Cannot cancel within cutoff window"
        );
        return Err(AppError::BadRequest(format!(
            "Cannot cancel within {} hours of the appointment. Please contact support for assistance.",
            cancel_window_hours
        )));
    }

    let mut work_order_active_model: work_orders::ActiveModel = work_order.clone().into();
    work_order_active_model.work_order_status_id = Set(target_closed_status_id);
    work_order_active_model.updated_at = Set(current_timestamp);

    let state_history_active_model = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(target_closed_status_id),
        changed_by_id: Set(requesting_customer_id),
        changed_at: Set(current_timestamp),
    };

    tracing::info!(
        reason = "CancelWorkOrderSuccess",
        work_order_id = %work_order.id,
        customer_id = %requesting_customer_id,
        message = "Successfully decided to cancel work order"
    );

    let cancel_reason_active_model = cancel_reasons::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        cancelled_by: Set(requesting_customer_id),
        reason: Set(reason),
        additional_comments: Set(additional_comments),
        created_at: Set(current_timestamp),
    };

    Ok(CancelWorkOrderEffect {
        work_order_model: work_order_active_model,
        state_history_model: state_history_active_model,
        cancel_reason_model: cancel_reason_active_model,
    })
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
            ward: "".to_string(),
            address: "".to_string(),
            building: None,
            appointment: Utc::now() + chrono::Duration::hours(48), // far future — passes 24h check
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            about_to_start_notified: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            chat_room_id: None,
        }
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_success() {
        let customer_id = Uuid::new_v4();
        let closed_status_id = 4;
        let work_order = dummy_work_order(customer_id, 1);

        let result = decide_cancel_work_order(
            work_order,
            closed_status_id,
            customer_id,
            24,
            "Changed my mind".to_string(),
            None,
        );
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(
            effect.work_order_model.work_order_status_id,
            Set(closed_status_id)
        );
    }

    #[ignore]
    #[test]
    fn test_decide_cancel_wrong_customer() {
        let customer_id = Uuid::new_v4();
        let wrong_customer_id = Uuid::new_v4();
        let work_order = dummy_work_order(customer_id, 1);

        let result = decide_cancel_work_order(
            work_order,
            4,
            wrong_customer_id,
            24,
            "Reason".to_string(),
            None,
        );
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
        let mut work_order = dummy_work_order(customer_id, 1);
        // Appointment is only 1 hour away — within the 24h window
        work_order.appointment = Utc::now() + chrono::Duration::hours(1);

        let result =
            decide_cancel_work_order(work_order, 4, customer_id, 24, "Reason".to_string(), None);
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
        let mut work_order = dummy_work_order(customer_id, 1);
        // Appointment is 25 hours away — just outside the 24h window
        work_order.appointment = Utc::now() + chrono::Duration::hours(25);

        let result = decide_cancel_work_order(
            work_order,
            4,
            customer_id,
            24,
            "Reason".to_string(),
            Some("Extra details".to_string()),
        );
        assert!(result.is_ok());
    }
}
