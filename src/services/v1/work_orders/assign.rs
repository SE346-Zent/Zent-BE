use std::collections::HashMap;
use chrono::{Duration, FixedOffset, TimeZone, Utc, Timelike};
use sea_orm::Set;
use crate::{
    core::errors::AppError,
    entities::work_orders,
    model::requests::work_orders::assign_request::AssignWorkOrderRequest,
};

pub struct AssignWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
}

pub fn decide_assign_work_order(
    req: AssignWorkOrderRequest,
    work_order: work_orders::Model,
    technician_work_orders: Vec<work_orders::Model>,
    policies: &HashMap<String, String>,
    assigned_status_id: i32,
    done_status_id: i32,
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

    let buffer_hours: i64 = policies.get("buffer")
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

    let buffer_duration = Duration::hours(buffer_hours);

    for other_wo in technician_work_orders {
        if other_wo.id == work_order.id {
            continue;
        }
        // Only conflict with non-completed work orders
        if other_wo.work_order_status_id == done_status_id {
            continue;
        }

        let diff = (other_wo.appointment - work_order.appointment).num_milliseconds();
        if diff.abs() < buffer_duration.num_milliseconds() {
            return Err(AppError::Conflict(format!(
                "Technician has an overlapping appointment (ID: {}) within the {}-hour buffer time",
                other_wo.work_order_number, buffer_hours
            )));
        }
    }

    let mut active_wo: work_orders::ActiveModel = work_order.into();
    active_wo.technician_id = Set(Some(req.technician_id));
    active_wo.work_order_status_id = Set(assigned_status_id);
    active_wo.updated_at = Set(Utc::now());

    Ok(AssignWorkOrderEffect { work_order: active_wo })
}
