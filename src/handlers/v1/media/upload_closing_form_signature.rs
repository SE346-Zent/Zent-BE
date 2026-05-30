use axum::{extract::{State, Path, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;
use crate::core::errors::AppError;
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::utils::{oci, geocoding};
use crate::entities::work_orders;
use crate::model::responses::base::ApiResponse;

#[utoipa::path(
    post,
    path = "/api/v1/media/work_orders/{id}/closing_form/signature",
    request_body(content_type = "multipart/form-data", description = "Signature image file and metadata (latitude, longitude)"),
    responses(
        (status = 200, description = "Signature uploaded successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
/// Handle multipart/form-data requests to upload a customer signature for a work order closing form.
///
/// This handler extracts the signature image data and location metadata, validates
/// security (technician assignment), time drift, and geofencing, uploads the
/// image to OCI, and returns the public access URL.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user (must be the assigned technician).
/// * `db_connection` - Shared database connection pool.
/// * `lookup_tables` - Shared in-memory reference tables (for policies).
/// * `work_order_id` - The unique ID of the work order from the URL path.
/// * `multipart_payload` - The raw multipart request body.
///
/// # Returns
/// A result containing the successful `ApiResponse` with the OCI access URL, or an `AppError`.
pub async fn upload_closing_form_signature(
    Extension(authenticated_user): Extension<AuthUser>,
    State(db_connection): State<Arc<DatabaseConnection>>,
    State(lookup_tables): State<Arc<LookupTables>>,
    Path(work_order_id): Path<Uuid>,
    mut multipart_payload: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let mut uploaded_file_data = None;
    let mut file_content_type = String::new();
    let mut original_file_name = String::new();
    let mut device_latitude = None;
    let mut device_longitude = None;
    let mut device_internet_time = None;

    while let Some(field) = multipart_payload.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "file" => {
                file_content_type = field.content_type().unwrap_or("image/png").to_string();
                original_file_name = field.file_name().unwrap_or("signature.png").to_string();
                uploaded_file_data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "latitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_latitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "longitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_longitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "internet_time" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                device_internet_time = Some(val.parse::<i64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let file_bytes = uploaded_file_data.ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;
    let latitude = device_latitude.ok_or_else(|| AppError::BadRequest("Latitude is missing".to_string()))?;
    let longitude = device_longitude.ok_or_else(|| AppError::BadRequest("Longitude is missing".to_string()))?;
    let internet_time = device_internet_time.ok_or_else(|| AppError::BadRequest("internet_time is missing".to_string()))?;

    // Internet time drift check
    let allowed_drift_minutes: i64 = lookup_tables.policies
        .get("internet_time_drift_minutes")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let drift_seconds = (chrono::Utc::now().timestamp() - internet_time).abs();
    if drift_seconds > allowed_drift_minutes * 60 {
        return Err(AppError::BadRequest(format!(
            "Device time is too far from server time ({} seconds drift, max {} minutes allowed). Please sync your device clock and try again.",
            drift_seconds, allowed_drift_minutes
        )));
    }

    let work_order_record = work_orders::Entity::find_by_id(work_order_id)
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if work_order_record.technician_id != Some(authenticated_user.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let site_location = geocoding::geocode_address(
        &work_order_record.address, &work_order_record.ward, &work_order_record.province, &work_order_record.country,
    ).await?;

    let geofence_radius_meters: f64 = lookup_tables.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok()).unwrap_or(500.0);

    let is_within_site = crate::utils::geo::is_within_geofence(
        latitude, longitude, site_location.lat, site_location.lng, geofence_radius_meters,
    );

    if !is_within_site {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let file_extension = original_file_name.split('.').next_back().unwrap_or("png");
    let generated_unique_name = format!(
        "{}/sig/{}.{}", work_order_id, chrono::Utc::now().timestamp(), file_extension
    );

    oci::upload_object(&generated_unique_name, file_bytes.to_vec(), &file_content_type).await?;

    let app_config = crate::core::config::AppConfig::get();
    let access_url = format!("{}{}", app_config.par_read_work_orders, generated_unique_name);

    Ok(Json(ApiResponse::success(200, "Signature uploaded successfully", access_url)))
}
