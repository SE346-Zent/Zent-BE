use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders},
};

pub struct ApproveRefusalEffect {
    pub work_order: work_orders::ActiveModel,
    pub reject_form: work_order_reject_forms::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

/// Admin approves the technician's refusal.
/// This means the admin agrees there is an anomaly.
/// Status changes to 'Rejected'.
pub fn decide_approve_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    admin_id: Uuid,
    rejected_status_id: i32,
) -> Result<ApproveRefusalEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Work order does not match this rejection form".to_string()));
    }

    let now = Utc::now();

    // 1. Mark form as approved
    let mut active_form: work_order_reject_forms::ActiveModel = reject_form.into();
    active_form.approved = Set(true);
    active_form.approver_id = Set(Some(admin_id));
    active_form.updated_at = Set(Some(now));

    // 2. Terminate Work Order as 'Rejected'
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(rejected_status_id);
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        work_order_status_id: Set(rejected_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(now),
    };

    Ok(ApproveRefusalEffect {
        work_order: active_wo,
        reject_form: active_form,
        state_history,
    })
}
