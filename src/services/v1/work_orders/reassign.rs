use crate::{
    core::errors::AppError,
    entities::{work_order_state_history, work_orders},
    model::requests::work_orders::reassign_request::ReassignWorkOrderRequest,
};
use chrono::{FixedOffset, Timelike, Utc};
use sea_orm::Set;
use std::collections::HashMap;
use uuid::Uuid;

/// Reuses the same effect shape as assign — identical fields.
pub use crate::services::v1::work_orders::assign::AssignWorkOrderEffect;

/// Pure logic: decide whether an admin can reassign a work order to a different technician.
///
/// Rules:
/// - The work order MUST already have a technician (opposite of assign)
/// - Appointment must fall within workday hours
/// - Work order must not be closed
/// - New technician must not have a time conflict
pub fn decide_reassign_work_order(
    req: ReassignWorkOrderRequest,
    work_order: work_orders::Model,
    technician_work_orders: Vec<work_orders::Model>,
    policies: &HashMap<String, String>,
    assigned_status_id: i32,
    done_status_id: i32,
    changed_by_id: Uuid,
) -> Result<AssignWorkOrderEffect, AppError> {
    // ── Precondition: must have an existing technician ─────────────
    if work_order.technician_id.is_none() {
        tracing::warn!(
            reason = "WorkOrderNotAssigned",
            work_order_id = %work_order.id,
            message = "Work order has no technician assigned — use assign instead"
        );
        return Err(AppError::BadRequest(
            "Work order has no technician assigned — use assign instead".into(),
        ));
    }

    let appointment_local = crate::utils::time::to_utc7_time(work_order.appointment);

    let workday_start: u32 = match policies.get("workday_start") {
        None => {
            tracing::error!(
                reason = "MissingWorkdayStartPolicy",
                work_order_id = %work_order.id,
                message = "Missing workday_start policy"
            );
            return Err(AppError::Internal(anyhow::anyhow!("Missing workday_start policy")));
        }
        Some(val) => match val.parse() {
            Err(_) => {
                tracing::error!(
                    reason = "InvalidWorkdayStartPolicy",
                    work_order_id = %work_order.id,
                    message = "Invalid workday_start policy"
                );
                return Err(AppError::Internal(anyhow::anyhow!("Invalid workday_start policy")));
            }
            Ok(parsed) => parsed,
        }
    };

    let workday_end: u32 = match policies.get("workday_end") {
        None => {
            tracing::error!(
                reason = "MissingWorkdayEndPolicy",
                work_order_id = %work_order.id,
                message = "Missing workday_end policy"
            );
            return Err(AppError::Internal(anyhow::anyhow!("Missing workday_end policy")));
        }
        Some(val) => match val.parse() {
            Err(_) => {
                tracing::error!(
                    reason = "InvalidWorkdayEndPolicy",
                    work_order_id = %work_order.id,
                    message = "Invalid workday_end policy"
                );
                return Err(AppError::Internal(anyhow::anyhow!("Invalid workday_end policy")));
            }
            Ok(parsed) => parsed,
        }
    };

    let hour = appointment_local.hour();
    if hour < workday_start || hour >= workday_end {
        tracing::warn!(
            reason = "AppointmentOutsideWorkdayLimits",
            work_order_id = %work_order.id,
            appointment = %work_order.appointment,
            hour = %hour,
            workday_start = %workday_start,
            workday_end = %workday_end,
            message = "Appointment hour is outside workday limits"
        );
        return Err(AppError::BadRequest(format!(
            "Appointment hour {:02}:{:02} is outside workday limits ({:02}:00 - {:02}:00)",
            hour,
            appointment_local.minute(),
            workday_start,
            workday_end
        )));
    }

    // Ensure we don't reassign a completed work order
    if work_order.work_order_status_id == done_status_id {
        tracing::warn!(
            reason = "CannotReassignCompletedWorkOrder",
            work_order_id = %work_order.id,
            done_status_id = %done_status_id,
            message = "Cannot reassign a completed work order"
        );
        return Err(AppError::BadRequest(
            "Cannot reassign a completed work order".into(),
        ));
    }

    // Don't reassign to the same technician
    if work_order.technician_id == Some(req.technician_id) {
        tracing::warn!(
            reason = "WorkOrderAlreadyAssignedToTechnician",
            work_order_id = %work_order.id,
            technician_id = %req.technician_id,
            message = "Work order is already assigned to this technician"
        );
        return Err(AppError::BadRequest(
            "Work order is already assigned to this technician".into(),
        ));
    }

    for other_wo in &technician_work_orders {
        if other_wo.id == work_order.id {
            continue;
        }
        if other_wo.work_order_status_id == done_status_id {
            continue;
        }
        if other_wo.appointment == work_order.appointment {
            tracing::warn!(
                reason = "TechnicianScheduleConflict",
                work_order_id = %work_order.id,
                technician_id = %req.technician_id,
                appointment = %work_order.appointment,
                message = "Technician already has an appointment at this exact time"
            );
            return Err(AppError::Conflict(
                "Technician already has an appointment at this exact time".into(),
            ));
        }
    }

    // ── Build effect ───────────────────────────────────────────────
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

    tracing::info!(
        reason = "ReassignWorkOrderSuccess",
        work_order_id = %work_order.id,
        technician_id = %req.technician_id,
        changed_by_id = %changed_by_id,
        message = "Successfully decided to reassign work order"
    );

    Ok(AssignWorkOrderEffect {
        work_order_model: active_wo,
        state_history_model: state_history,
    })
}
