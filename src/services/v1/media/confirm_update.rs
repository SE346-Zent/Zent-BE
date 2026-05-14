use std::collections::HashMap;
use sea_orm::Set;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use crate::core::errors::AppError;
use crate::model::requests::media::confirm_update_request::ConfirmUpdateRequest;
use crate::entities::{work_orders, work_order_image_links};
use crate::utils::geo::is_within_geofence;

pub struct ConfirmUpdateEffect {
    pub image_id: Uuid,
    pub object_name: String,
    pub internet_time: i64,
    pub updated_at: DateTime<Utc>,
    pub link_update: work_order_image_links::ActiveModel,
}

pub fn decide_confirm_update(
    req: ConfirmUpdateRequest,
    work_order: &work_orders::Model,
    image_id: Uuid,
    existing_link: work_order_image_links::Model,
    technician_id: Uuid,
    target_lat: f64,
    target_lng: f64,
    object_name: String,
    policies: &HashMap<String, String>,
) -> Result<ConfirmUpdateEffect, AppError> {
    // 1. Security Check
    if work_order.technician_id != Some(technician_id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // 2. Internet time drift check
    let drift_minutes: i64 = policies
        .get("internet_time_drift_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let now = Utc::now();
    let drift_seconds = (now.timestamp() - req.internet_time).abs();
    if drift_seconds > drift_minutes * 60 {
        return Err(AppError::BadRequest(format!(
            "Device time is too far from server time ({} seconds drift, max {} minutes allowed). Please sync your device clock and try again.",
            drift_seconds, drift_minutes
        )));
    }

    // 3. Geofencing Check
    let radius: f64 = policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = is_within_geofence(
        req.latitude,
        req.longitude,
        target_lat,
        target_lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 4. Prepare Side-Effects
    let mut link_active: work_order_image_links::ActiveModel = existing_link.into();
    link_active.latitude = Set(Some(req.latitude));
    link_active.longitude = Set(Some(req.longitude));
    link_active.is_verified = Set(true);

    Ok(ConfirmUpdateEffect {
        image_id,
        object_name,
        internet_time: req.internet_time,
        updated_at: now,
        link_update: link_active,
    })
}
