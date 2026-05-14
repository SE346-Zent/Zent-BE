use std::collections::HashMap;
use sea_orm::Set;
use uuid::Uuid;
use chrono::Utc;
use crate::core::errors::AppError;
use crate::entities::{work_orders, images, work_order_image_links};
use crate::model::requests::media::confirm_upload_request::ConfirmUploadRequest;
use crate::utils::geo::is_within_geofence;

pub struct ConfirmUploadEffect {
    pub image: images::ActiveModel,
    pub image_link: work_order_image_links::ActiveModel,
}

pub fn decide_confirm_upload(
    req: ConfirmUploadRequest,
    work_order: &work_orders::Model,
    technician_id: Uuid,
    target_lat: f64,
    target_lng: f64,
    object_name: String,
    policies: &HashMap<String, String>,
) -> Result<ConfirmUploadEffect, AppError> {
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
    let image_id = Uuid::new_v4();

    let image = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(object_name),
        internet_time: Set(Some(req.internet_time)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let image_link = work_order_image_links::ActiveModel {
        image_id: Set(image_id),
        work_order_id: Set(work_order.id),
        phase: Set(req.phase),
        latitude: Set(Some(req.latitude)),
        longitude: Set(Some(req.longitude)),
        is_verified: Set(true),
    };

    Ok(ConfirmUploadEffect { image, image_link })
}
