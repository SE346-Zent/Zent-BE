use crate::{
    core::errors::AppError,
    entities::{work_order_state_history, work_orders},
    model::requests::work_orders::assign_request::AssignWorkOrderRequest,
};
use chrono::{FixedOffset, Timelike, Utc};
use sea_orm::Set;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug)]
pub struct AssignWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_assign_work_order(
    req: AssignWorkOrderRequest,
    work_order: work_orders::Model,
    technician_work_orders: Vec<work_orders::Model>,
    policies: &HashMap<String, String>,
    assigned_status_id: i32,
    done_status_id: i32,
    changed_by_id: Uuid,
) -> Result<AssignWorkOrderEffect, AppError> {
    let tz_offset = FixedOffset::east_opt(7 * 3600).unwrap(); // GMT+7
    let appointment_local = work_order.appointment.with_timezone(&tz_offset);

    let workday_start: u32 = policies
        .get("workday_start")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing workday_start policy")))?
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid workday_start policy")))?;

    let workday_end: u32 = policies
        .get("workday_end")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing workday_end policy")))?
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid workday_end policy")))?;

    let _buffer_hours: i64 = policies
        .get("buffer")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5); // Default to 5 as per policy

    let hour = appointment_local.hour();
    if hour < workday_start || hour >= workday_end {
        return Err(AppError::BadRequest(format!(
            "Appointment hour {:02}:{:02} is outside workday limits ({:02}:00 - {:02}:00)",
            hour,
            appointment_local.minute(),
            workday_start,
            workday_end
        )));
    }

    // Ensure we don't assign a completed or rejected work order
    if work_order.work_order_status_id == done_status_id {
        return Err(AppError::BadRequest(
            "Cannot assign a completed work order".into(),
        ));
    }

    for other_wo in technician_work_orders {
        if other_wo.id == work_order.id {
            continue;
        }
        // Only conflict with non-completed work orders
        if other_wo.work_order_status_id == done_status_id {
            continue;
        }

        if other_wo.appointment == work_order.appointment {
            return Err(AppError::Conflict(
                "Technician already has an appointment at this exact time".into(),
            ));
        }
    }

    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.technician_id = Set(Some(req.technician_id));
    active_wo.work_order_status_id = Set(assigned_status_id);
    active_wo.updated_at = Set(Utc::now());

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(assigned_status_id),
        changed_by_id: Set(changed_by_id),
        changed_at: Set(Utc::now()),
    };

    Ok(AssignWorkOrderEffect {
        work_order: active_wo,
        state_history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dummy_work_order() -> work_orders::Model {
        work_orders::Model {
            id: Uuid::new_v4(),
            work_order_status_id: 1, // Pending
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
            appointment: Utc.with_ymd_and_hms(2026, 1, 1, 3, 0, 0).unwrap(), // 10:00 AM GMT+7
            admin_id: None,
            technician_id: None,
            complete_form_id: None,
            work_order_number: "".to_string(),
            reject_form_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn test_decide_assign_work_order_success() {
        let wo = dummy_work_order();
        let tech_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let req = AssignWorkOrderRequest {
            technician_id: tech_id,
        };
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("workday_end".to_string(), "17".to_string());

        let result = decide_assign_work_order(req, wo, vec![], &policies, 2, 4, admin_id);
        assert!(result.is_ok());
        let effect = result.unwrap();

        assert_eq!(effect.work_order.technician_id, Set(Some(tech_id)));
        assert_eq!(effect.work_order.work_order_status_id, Set(2)); // Assigned
        assert_eq!(effect.state_history.to_status_id, Set(2));
    }

    #[test]
    fn test_decide_assign_work_order_conflict() {
        let wo1 = dummy_work_order();
        let mut wo2 = dummy_work_order();
        wo2.id = Uuid::new_v4();
        // wo2 has the same appointment time

        let tech_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let req = AssignWorkOrderRequest {
            technician_id: tech_id,
        };
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("workday_end".to_string(), "17".to_string());

        let result = decide_assign_work_order(req, wo1, vec![wo2], &policies, 2, 4, admin_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Conflict(_) => {}
            _ => panic!("Expected Conflict error"),
        }
    }

    #[test]
    fn test_decide_assign_work_order_outside_workhours() {
        let mut wo = dummy_work_order();
        wo.appointment = Utc.with_ymd_and_hms(2026, 1, 1, 15, 0, 0).unwrap(); // 22:00 GMT+7

        let tech_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let req = AssignWorkOrderRequest {
            technician_id: tech_id,
        };
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("workday_end".to_string(), "17".to_string());

        let result = decide_assign_work_order(req, wo, vec![], &policies, 2, 4, admin_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => assert!(msg.contains("outside workday limits")),
            _ => panic!("Expected BadRequest"),
        }
    }
    #[test]
    fn test_decide_assign_work_order_already_completed() {
        let mut wo = dummy_work_order();
        let done_status_id = 4;
        wo.work_order_status_id = done_status_id;

        let tech_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let req = AssignWorkOrderRequest {
            technician_id: tech_id,
        };
        let mut policies = HashMap::new();
        policies.insert("workday_start".to_string(), "8".to_string());
        policies.insert("workday_end".to_string(), "17".to_string());

        let result =
            decide_assign_work_order(req, wo, vec![], &policies, 2, done_status_id, admin_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::BadRequest(msg) => {
                assert_eq!(msg, "Cannot assign a completed work order")
            }
            _ => panic!("Expected BadRequest for completed work order"),
        }
    }

    #[test]
    fn test_decide_assign_work_order_missing_policies() {
        let wo = dummy_work_order();
        let tech_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        let req = AssignWorkOrderRequest {
            technician_id: tech_id,
        };
        let policies = HashMap::new(); // Missing workday_start/end

        let result = decide_assign_work_order(req, wo, vec![], &policies, 2, 4, admin_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::Internal(err) => {
                assert!(err.to_string().contains("Missing workday_start policy"))
            }
            _ => panic!("Expected Internal error for missing policy"),
        }
    }
}
