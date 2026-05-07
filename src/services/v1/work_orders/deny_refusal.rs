use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_reject_forms, work_order_state_history, work_orders},
};

pub struct DenyRefusalEffect {
    pub work_order: work_orders::ActiveModel,
    pub reject_form: work_order_reject_forms::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub fn decide_deny_refusal(
    work_order: work_orders::Model,
    reject_form: work_order_reject_forms::Model,
    admin_id: Uuid,
    rejected_status_id: i32,
) -> Result<DenyRefusalEffect, AppError> {
    if work_order.reject_form_id != Some(reject_form.id) {
        return Err(AppError::BadRequest("Work order does not match this rejection form".to_string()));
    }

    let now = Utc::now();

    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    // Revert status to Rejected
    active_wo.work_order_status_id = Set(rejected_status_id);
    active_wo.updated_at = Set(now);

    let mut active_form: work_order_reject_forms::ActiveModel = reject_form.into();
    active_form.approved = Set(false);
    active_form.approver_id = Set(Some(admin_id));
    active_form.updated_at = Set(Some(now));

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        work_order_status_id: Set(rejected_status_id),
        changed_by_id: Set(admin_id),
        changed_at: Set(now),
    };

    Ok(DenyRefusalEffect {
        work_order: active_wo,
        reject_form: active_form,
        state_history,
    })
}
