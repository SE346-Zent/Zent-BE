use std::collections::HashMap;
use chrono::{FixedOffset, Utc, Timelike};
use sea_orm::Set;
use uuid::Uuid;
use crate::{
    core::errors::AppError,
    entities::{work_orders, work_order_state_history},
    model::requests::work_orders::assign_request::AssignWorkOrderRequest,
};

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

    let workday_start: u32 = policies.get("workday_start")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing workday_start policy")))?
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid workday_start policy")))?;

    let workday_end: u32 = policies.get("workday_end")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Missing workday_end policy")))?
        .parse()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("Invalid workday_end policy")))?;

    let _buffer_hours: i64 = policies.get("buffer")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5); // Default to 5 as per policy

    let hour = appointment_local.hour();
    if hour < workday_start || hour >= workday_end {
        return Err(AppError::BadRequest(format!(
            "Appointment hour {:02}:{:02} is outside workday limits ({:02}:00 - {:02}:00)",
            hour, appointment_local.minute(), workday_start, workday_end
        )));
    }

    // Ensure we don't assign a completed or rejected work order
    if work_order.work_order_status_id == done_status_id {
        return Err(AppError::BadRequest("Cannot assign a completed work order".into()));
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
            return Err(AppError::Conflict("Technician already has an appointment at this exact time".into()));
        }
    }

    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.technician_id = Set(Some(req.technician_id));
    active_wo.work_order_status_id = Set(assigned_status_id);
    active_wo.updated_at = Set(Utc::now());

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        work_order_status_id: Set(assigned_status_id),
        changed_by_id: Set(changed_by_id),
        changed_at: Set(Utc::now()),
    };

    Ok(AssignWorkOrderEffect { work_order: active_wo, state_history })
}
