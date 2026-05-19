use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_appointment_changes, work_orders},
};

#[derive(Debug)]
pub struct ChangeAppointmentEffect {
    pub work_order: work_orders::ActiveModel,
    pub audit: work_order_appointment_changes::ActiveModel,
}

/// Pure logic: decide whether an admin can change the appointment of a work order.
///
/// Rules:
/// - Only when work order status is "Pending" or "Assigned"
/// - No reason required — just the new appointment that overwrites the existing one
pub fn decide_change_appointment(
    work_order: work_orders::Model,
    new_appointment: chrono::DateTime<Utc>,
    pending_status_id: i32,
    assigned_status_id: i32,
    changed_by_id: Uuid,
) -> Result<ChangeAppointmentEffect, AppError> {
    // Only allow changes when status is Pending or Assigned
    if work_order.work_order_status_id != pending_status_id
        && work_order.work_order_status_id != assigned_status_id
    {
        return Err(AppError::BadRequest(
            "Appointment can only be changed when the work order is Pending or Assigned".into(),
        ));
    }

    let now = Utc::now();
    let old_appointment = work_order.appointment;

    // Update the work order
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.appointment = Set(new_appointment);
    active_wo.updated_at = Set(now);

    // Audit row
    let audit = work_order_appointment_changes::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        old_appointment: Set(old_appointment),
        new_appointment: Set(new_appointment),
        changed_by_id: Set(changed_by_id),
        created_at: Set(now),
    };


    Ok(ChangeAppointmentEffect {
        work_order: active_wo,
        audit
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_work_order(status_id: i32) -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: status_id,
            customer_id: Uuid::new_v4(),
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
            appointment: Utc::now(),
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
            chat_room_id: None,
        }
    }

    #[test]
    fn test_change_appointment_pending() {
        let wo = dummy_work_order(1); // Pending
        let old_appointment = wo.appointment; // capture before wo is consumed
        let new_appt = Utc::now() + chrono::Duration::days(1);
        let result = decide_change_appointment(wo, new_appt, 1, 2, Uuid::new_v4());
        assert!(result.is_ok());
        let effect = result.unwrap();
        assert_eq!(effect.work_order.appointment, Set(new_appt));
        assert_eq!(effect.audit.old_appointment, Set(old_appointment));
    }

    #[test]
    fn test_change_appointment_assigned() {
        let wo = dummy_work_order(2); // Assigned
        let new_appt = Utc::now() + chrono::Duration::days(2);
        let result = decide_change_appointment(wo, new_appt, 1, 2, Uuid::new_v4());
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_appointment_rejected_when_in_progress() {
        let wo = dummy_work_order(3); // InProg
        let new_appt = Utc::now() + chrono::Duration::days(1);
        let result = decide_change_appointment(wo, new_appt, 1, 2, Uuid::new_v4());
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert!(msg.contains("Pending or Assigned"));
            }
            _ => panic!("Expected BadRequest"),
        }
    }
}
