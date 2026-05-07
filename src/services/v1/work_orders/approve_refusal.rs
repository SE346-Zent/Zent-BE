use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders, users},
};

pub struct ApproveRefusalEffect {
    pub work_order: work_orders::ActiveModel,
    pub reject_form: work_order_reject_forms::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_approve_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    technician: users::Model,
    technician_work_orders: Vec<work_orders::Model>,
    admin_id: Uuid,
    assigned_status_id: i32,
    done_status_id: i32,
) -> Result<ApproveRefusalEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Work order does not match this rejection form".to_string()));
    }

    if work_order.technician_id == Some(technician.id) {
        return Err(AppError::BadRequest("Cannot reassign to the same technician who refused".to_string()));
    }

    // Similar logic as manual assign
    for other_wo in technician_work_orders {
        if other_wo.id == work_order.id {
            continue;
        }
        if other_wo.work_order_status_id == done_status_id {
            continue;
        }
        if other_wo.appointment == work_order.appointment {
            return Err(AppError::Conflict("Technician already has an appointment at this exact time".into()));
        }
    }

    let now = Utc::now();

    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.technician_id = Set(Some(technician.id));
    active_wo.work_order_status_id = Set(assigned_status_id);
    active_wo.updated_at = Set(now);

    let mut active_form: work_order_reject_forms::ActiveModel = reject_form.into();
    active_form.approved = Set(true);
    active_form.approver_id = Set(Some(admin_id));
    active_form.updated_at = Set(Some(now));

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        work_order_status_id: Set(assigned_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(now),
    };

    Ok(ApproveRefusalEffect {
        work_order: active_wo,
        reject_form: active_form,
        state_history,
    })
}
