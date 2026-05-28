use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_orders, work_order_ratings},
};

#[derive(Debug, Clone)]
pub struct RateWorkOrderEffect {
    pub rating_model: work_order_ratings::ActiveModel,
}

/// Pure logic: decide whether a customer can rate a work order.
pub fn decide_rate_work_order(
    work_order: work_orders::Model,
    customer_id: Uuid,
    closed_status_id: i32,
    rating: i32,
    comment: Option<String>,
    rating_already_exists: bool,
) -> Result<RateWorkOrderEffect, AppError> {
    // Only the owner of the work order can rate
    if work_order.customer_id != customer_id {
        return Err(AppError::Forbidden("You can only rate your own work orders".to_string()));
    }

    // Only closed work orders can be rated
    if work_order.work_order_status_id != closed_status_id {
        return Err(AppError::BadRequest("Only closed work orders can be rated".to_string()));
    }

    // A work order can only be rated once
    if rating_already_exists {
        return Err(AppError::BadRequest("A rating has already been submitted for this work order".to_string()));
    }

    let current_timestamp = Utc::now();
    let rating_model = work_order_ratings::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        rating: Set(rating),
        comment: Set(comment),
        created_at: Set(current_timestamp),
        updated_at: Set(current_timestamp),
    };

    Ok(RateWorkOrderEffect { rating_model })
}
