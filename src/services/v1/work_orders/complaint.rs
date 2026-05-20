use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::work_orders,
};

#[derive(Debug)]
pub struct ComplaintWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
}

/// Pure logic: decide whether a customer can file a complaint on their work order.
///
/// Rules:
/// - Only the owner of the work order can complain
/// - Cannot complain on a closed work order
/// - Cannot submit multiple complaints (one per work order)
pub fn decide_complaint_work_order(
    work_order: work_orders::Model,
    customer_id: Uuid,
    closed_status_id: i32,
    message: String,
) -> Result<ComplaintWorkOrderEffect, AppError> {
    // Only the owner of the work order can complain
    if work_order.customer_id != customer_id {
        return Err(AppError::Forbidden("You can only complain about your own work orders".to_string()));
    }

    // Cannot complain on a closed work order
    if work_order.work_order_status_id == closed_status_id {
        return Err(AppError::BadRequest("Cannot complain on a closed work order".to_string()));
    }

    // One complaint per work order
    if work_order.customer_complaint.is_some() {
        return Err(AppError::BadRequest("A complaint has already been submitted for this work order".to_string()));
    }

    let mut active_wo: work_orders::ActiveModel = work_order.into();
    active_wo.customer_complaint = Set(Some(message));
    active_wo.customer_complaint_at = Set(Some(Utc::now()));
    active_wo.updated_at = Set(Utc::now());

    Ok(ComplaintWorkOrderEffect { work_order: active_wo })
}
