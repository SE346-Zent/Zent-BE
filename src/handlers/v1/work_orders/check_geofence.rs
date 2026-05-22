use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::geofence_check_request::GeofenceCheckRequest;
use crate::model::responses::work_orders::geofence_check_response::GeofenceCheckResponse;
use crate::model::responses::base::ApiResponse;
use geo::prelude::*;
use geo::Point;

/// Validate technician's proximity to the work site geofence.
#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/geofence",
    request_body = GeofenceCheckRequest,
    responses(
        (status = 200, description = "Geofence checked successfully", body = ApiResponse<GeofenceCheckResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 404, description = "Work order not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn check_geofence(
    Extension(_auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<GeofenceCheckRequest>,
) -> Result<Json<ApiResponse<GeofenceCheckResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Load cached or DB work order model
    let wo = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    // Geocode the work order address
    let target = crate::utils::geocoding::geocode_address(
        &wo.address,
        &wo.city,
        &wo.province,
        &wo.country,
    ).await?;

    // Retrieve allowed radius from policies (default to 2000 meters)
    let radius: f64 = luts.policies
        .get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000.0);

    // Calculate distance
    let tech_point = Point::new(payload.longitude, payload.latitude);
    let target_point = Point::new(target.lng, target.lat);
    
    #[allow(deprecated)]
    let distance_meters = tech_point.haversine_distance(&target_point);
    let is_within = distance_meters <= radius;

    Ok(Json(ApiResponse::success(
        200,
        "Geofence checked successfully",
        GeofenceCheckResponse {
            is_within,
            distance_meters,
            allowed_radius_meters: radius,
        },
    )))
}
