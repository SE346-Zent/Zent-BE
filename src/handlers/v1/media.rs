use axum::{
    extract::{State, Path, Multipart},
    http::StatusCode,
    Json,
    Extension,
};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, TransactionTrait, ActiveModelTrait, Set};
use uuid::Uuid;

use crate::core::errors::AppError;
use crate::core::lookup_tables::LookupTables;
use crate::extractor::auth_user::AuthUser;
use crate::utils::{oci, geocoding};
use crate::entities::{work_orders, work_order_image_links, images};
use crate::services::v1::media::{confirm_upload, confirm_update};
use crate::model::responses::base::ApiResponse;
use crate::model::requests::media::confirm_upload_request::ConfirmUploadRequest;
use crate::model::requests::media::confirm_update_request::ConfirmUpdateRequest;

#[utoipa::path(
    post,
    path = "/api/v1/media/work_orders/{id}/closing_form/photos",
    request_body(content_type = "multipart/form-data", description = "Image file and metadata (latitude, longitude, phase)"),
    responses(
        (status = 200, description = "Photo uploaded successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_closing_form_photo(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();
    let mut latitude = None;
    let mut longitude = None;
    let mut phase = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("upload.jpg").to_string();
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
            "phase" => {
                phase = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;
    let latitude = latitude.ok_or_else(|| AppError::BadRequest("Latitude is missing".to_string()))?;
    let longitude = longitude.ok_or_else(|| AppError::BadRequest("Longitude is missing".to_string()))?;

    if phase.is_empty() {
        return Err(AppError::BadRequest("Phase is missing".to_string()));
    }

    // 1. Fetch data (I/O)
    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // 2. Authorization — only assigned technician (or superadmin/admin)
    if work_order.technician_id != Some(auth.user.id)
        && auth.role.name != "SuperAdmin"
        && auth.role.name != "Admin"
    {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // 3. Geofencing check (before OCI upload to avoid orphaned objects)
    let target_location = geocoding::geocode_address(
        &work_order.address,
        &work_order.city,
        &work_order.province,
        &work_order.country,
    ).await?;

    let radius: f64 = luts.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        latitude,
        longitude,
        target_location.lat,
        target_location.lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 4. Generate Unique File Name and Upload to OCI
    let extension = file_name.split('.').last().unwrap_or("jpg");
    let unique_file_name = format!(
        "wo_closing_{}_{}_{}.{}", 
        id, 
        chrono::Utc::now().timestamp(), 
        Uuid::new_v4(), 
        extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    // 5. Decision Logic (Pure)
    let payload = ConfirmUploadRequest {
        unique_file_name,
        latitude,
        longitude,
        phase,
    };

    let effect = confirm_upload::decide_confirm_upload(
        payload,
        &work_order,
        auth.user.id,
        target_location.lat,
        target_location.lng,
        object_name,
        &luts.policies,
    )?;

    // 6. Execution (I/O)
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.image.insert(txn).await?;
            effect.image_link.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo uploaded successfully")))
}

#[utoipa::path(
    patch,
    path = "/api/v1/media/work_orders/{id}/closing_form/photos/{image_id}",
    request_body(content_type = "multipart/form-data", description = "New image file and metadata (latitude, longitude)"),
    responses(
        (status = 200, description = "Photo updated successfully"),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden/Geofencing violation"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_closing_form_photo(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    Path((id, image_id)): Path<(Uuid, Uuid)>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();
    let mut latitude = None;
    let mut longitude = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("update.jpg").to_string();
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
    let latitude = latitude.ok_or_else(|| AppError::BadRequest("Latitude is missing".to_string()))?;
    let longitude = longitude.ok_or_else(|| AppError::BadRequest("Longitude is missing".to_string()))?;

    // 1. Fetch data (I/O)
    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // 2. Authorization — only assigned technician (or superadmin/admin)
    if work_order.technician_id != Some(auth.user.id)
        && auth.role.name != "SuperAdmin"
        && auth.role.name != "Admin"
    {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    let link = work_order_image_links::Entity::find_by_id((image_id, id))
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Image link not found".to_string()))?;

    // 3. Geofencing check (before OCI upload to avoid orphaned objects)
    let target_location = geocoding::geocode_address(
        &work_order.address,
        &work_order.city,
        &work_order.province,
        &work_order.country,
    ).await?;

    let radius: f64 = luts.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        latitude,
        longitude,
        target_location.lat,
        target_location.lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 4. Generate Unique File Name and Upload to OCI
    let extension = file_name.split('.').last().unwrap_or("jpg");
    let unique_file_name = format!(
        "wo_closing_update_{}_{}_{}.{}", 
        id, 
        chrono::Utc::now().timestamp(), 
        Uuid::new_v4(), 
        extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    // 5. Decision Logic (Pure)
    let payload = ConfirmUpdateRequest {
        unique_file_name,
        latitude,
        longitude,
    };

    let effect = confirm_update::decide_confirm_update(
        payload,
        &work_order,
        link,
        auth.user.id,
        target_location.lat,
        target_location.lng,
        object_name,
        &luts.policies,
    )?;

    // 6. Execution (I/O)
    db.transaction::<_, (), AppError>(|txn| {
        let new_image = effect.new_image;
        let link_update = effect.link_update;

        Box::pin(async move {
            // Insert new image record for the replacement photo
            new_image.insert(txn).await?;

            // Update link to point to the new image
            link_update.update(txn).await?;

            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Photo updated successfully")))
}

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

    // 1. Fetch data (I/O)
    let work_order = work_orders::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // Security Check
    if work_order.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // Geofencing Check
    let target_location = geocoding::geocode_address(
        &work_order.address,
        &work_order.city,
        &work_order.province,
        &work_order.country,
    ).await?;

    let radius: f64 = luts.policies.get("geofencing_radius")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500.0);

    let is_verified = crate::utils::geo::is_within_geofence(
        req_latitude,
        req_longitude,
        target_location.lat,
        target_location.lng,
        radius,
    );

    if !is_verified {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    // 2. Generate Unique File Name and Upload to OCI
    let extension = file_name.split('.').last().unwrap_or("png");
    let unique_file_name = format!(
        "sig_{}_{}_{}.{}", 
        id, 
        chrono::Utc::now().timestamp(), 
        Uuid::new_v4(), 
        extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    // 3. Prepare image + image_link for signature phase
    let now = chrono::Utc::now();
    let image_id = Uuid::new_v4();

    let image = images::ActiveModel {
        id: Set(image_id),
        object_name: Set(object_name),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let image_link = work_order_image_links::ActiveModel {
        image_id: Set(image_id),
        work_order_id: Set(work_order.id),
        phase: Set("signature".to_string()),
        latitude: Set(Some(req_latitude)),
        longitude: Set(Some(req_longitude)),
        is_verified: Set(is_verified),
    };

    // 4. Execute transaction
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            image.insert(txn).await?;
            image_link.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Generate full URL for the signature since it's returned directly
    let cfg = crate::core::config::AppConfig::get();
    let access_url = format!("{}{}", cfg.par_read_work_orders, unique_file_name);

    Ok(Json(ApiResponse::success(200, "Signature uploaded successfully", access_url)))
}

pub async fn get_work_order_photo(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn list_work_order_photos(
    State(_db): State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

#[utoipa::path(
    post,
    path = "/api/v1/media/new_part_forms/{id}/photos",
    request_body(content_type = "multipart/form-data", description = "New part image file"),
    responses(
        (status = 200, description = "Photo uploaded successfully"),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Form not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_new_part_photo(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let mut file_data = None;
    let mut content_type = String::new();
    let mut file_name = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "file" => {
                content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                file_name = field.file_name().unwrap_or("part_photo.jpg").to_string();
                file_data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            _ => {}
        }
    }

    let file_data = file_data.ok_or_else(|| AppError::BadRequest("File is missing".to_string()))?;

    // 1. Fetch data
    let form = crate::entities::new_part_forms::Entity::find_by_id(id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("New part form not found".to_string()))?;

    // 1b. Ownership check — only the assigned technician may upload
    let work_order = work_orders::Entity::find_by_id(form.work_order_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Parent work order not found".to_string()))?;

    if work_order.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // 2. Upload to OCI
    let extension = file_name.split('.').last().unwrap_or("jpg");
    let unique_file_name = format!(
        "new_part_{}_{}_{}.{}", 
        id, 
        chrono::Utc::now().timestamp(), 
        Uuid::new_v4(), 
        extension
    );

    let object_name = oci::upload_object(&unique_file_name, file_data.to_vec(), &content_type).await?;

    // 3. Decision Logic
    let effect = crate::services::v1::media::upload_new_part_photo::decide_upload_new_part_photo(
        &form,
        object_name,
    )?;

    // 4. Execution
    db.transaction::<_, (), AppError>(|txn| {
        Box::pin(async move {
            effect.image.insert(txn).await?;
            effect.image_link.insert(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    Ok(Json(ApiResponse::message_only(200, "Part photo uploaded successfully")))
}

