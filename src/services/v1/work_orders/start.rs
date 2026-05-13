use std::collections::HashMap;
use chrono::Utc;
use sea_orm::Set;
use uuid::Uuid;

use crate::{
    core::errors::AppError,
    entities::{work_order_state_history, work_orders},
    model::requests::work_orders::start_request::StartWorkOrderRequest,
    utils::geocoding,
};

pub struct StartWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
    pub state_history: work_order_state_history::ActiveModel,
}

pub async fn decide_start(
    payload: StartWorkOrderRequest,
    work_order: work_orders::Model,
    technician_id: Uuid,
    in_progress_status_id: i32,
    policies: &HashMap<String, String>,
) -> Result<StartWorkOrderEffect, AppError> {
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // Geofencing Check
    let target_location = geocoding::geocode_address(
        &work_order.address,
        &work_order.city,
        &work_order.province,
        &work_order.country,
    ).await?;

    let radius: f64 = policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        payload.latitude,
        payload.longitude,
        target_location.lat,
        target_location.lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let now = Utc::now();
    let mut active_wo: work_orders::ActiveModel = work_order.clone().into();
    active_wo.work_order_status_id = Set(in_progress_status_id);
    active_wo.updated_at = Set(now);

    let state_history = work_order_state_history::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_id: Set(work_order.id),
        from_status_id: Set(Some(work_order.work_order_status_id)),
        to_status_id: Set(in_progress_status_id),
        changed_by_id: Set(technician_id),
        changed_at: Set(now),
    };

    Ok(StartWorkOrderEffect {
        work_order: active_wo,
        state_history,
    })
}
