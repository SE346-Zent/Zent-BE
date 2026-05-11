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
pub async fn upload_closing_form_signature(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();
    let mut latitude = None;
    let mut longitude = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/png").to_string();
                file_name = field.file_name().unwrap_or("signature.png").to_string();
                file_data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "latitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                latitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "longitude" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                longitude = Some(val.parse::<f64>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;
    let req_latitude = latitude.ok_or_else(|| AppError::BadRequest("Latitude is missing".to_string()))?;
    let req_longitude = longitude.ok_or_else(|| AppError::BadRequest("Longitude is missing".to_string()))?;

    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if work_order.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let target_location = geocoding::geocode_address(
        &work_order.address, &work_order.city, &work_order.province, &work_order.country,
    ).await?;

    let radius: f64 = luts.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok()).unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        req_latitude, req_longitude, target_location.lat, target_location.lng, radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let extension = file_name.split('.').last().unwrap_or("png");
    let unique_file_name = format!(
        "{}/sig/{}.{}", id, chrono::Utc::now().timestamp(), extension
    );

    oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    let cfg = crate::core::config::AppConfig::get();
    let access_url = format!("{}{}", cfg.par_read_work_orders, unique_file_name);

    Ok(Json(ApiResponse::success(200, "Signature uploaded successfully", access_url)))
}
