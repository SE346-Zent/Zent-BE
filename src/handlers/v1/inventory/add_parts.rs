use axum::{extract::{State, Path, Multipart}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use crate::core::config::AppConfig;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::responses::base::{ApiResponse, MessageOnlyResponse};
use crate::entities::{work_orders as work_orders_ent, users};
use crate::utils::oci;

#[utoipa::path(
    post,
    path = "/api/v1/inventory/work_orders/{id}/parts",
    request_body(content_type = "multipart/form-data", description = "Part metadata and image files"),
    tag = "inventory",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the work order")
    ),
    responses(
        (status = 200, description = "Parts added successfully", body = MessageOnlyResponse),
        (status = 400, description = "Bad Request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"),
        (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
/// Handle requests from technicians to register new parts against a specific work order.
///
/// This handler verifies the work order exists, validates that the requesting 
/// technician is assigned to it, uploads images to OCI, and performs a multi-table
/// database transaction to persist the part registration form and any associated photo records.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user (must be the assigned technician).
/// * `db_connection` - Shared database connection pool.
/// * `work_order_id` - The unique ID of the work order to which parts are being added.
/// * `multipart_payload` - The multipart request containing part metadata and image files.
///
/// # Returns
/// A result containing a successful message-only `ApiResponse`, or an `AppError`.
pub async fn add_parts(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(mongodb): State<Arc<mongodb::Database>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    mut multipart_payload: Multipart,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Parse multipart fields
    let mut part_number = None;
    let mut part_types_id = None;
    let mut model_code = None;
    let mut serial_number = None;
    let mut description = None;
    let mut photo_files: Vec<(String, String, Vec<u8>)> = Vec::new(); // (filename, content_type, bytes)

    while let Some(field) = multipart_payload.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let field_name = field.name().unwrap_or_default().to_string();
        match field_name.as_str() {
            "partNumber" => {
                part_number = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "partTypesId" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                part_types_id = Some(val.parse::<i32>().map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "modelCode" => {
                model_code = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "serialNumber" => {
                serial_number = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "description" => {
                description = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
            }
            "photos" => {
                let file_name = field.file_name().unwrap_or("upload.jpg").to_string();
                let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                let bytes = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                photo_files.push((file_name, content_type, bytes.to_vec()));
            }
            _ => {}
        }
    }

    // Validate required fields
    let part_number = part_number.ok_or_else(|| AppError::BadRequest("Part number is required".to_string()))?;
    let part_types_id = part_types_id.ok_or_else(|| AppError::BadRequest("Part type is required".to_string()))?;
    let serial_number = serial_number.ok_or_else(|| AppError::BadRequest("Serial number is required".to_string()))?;

    if part_number.is_empty() {
        return Err(AppError::BadRequest("Part number cannot be empty".to_string()));
    }
    if serial_number.is_empty() {
        return Err(AppError::BadRequest("Serial number cannot be empty".to_string()));
    }
    if photo_files.len() > 5 {
        return Err(AppError::BadRequest("Maximum 5 photos allowed".to_string()));
    }

    // Verify work order exists and technician is assigned
    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if wo.technician_id != Some(auth.user.id) {
        return Err(AppError::Forbidden("You are not assigned to this work order".to_string()));
    }

    // Capture fields before wo is moved
    let wo_id = wo.id;
    let wo_number = wo.work_order_number.clone();
    let wo_province = wo.province.clone();

    // Upload images to OCI and collect object names
    let mut uploaded_object_names: Vec<String> = Vec::new();
    for (file_name, content_type, bytes) in photo_files {
        let file_extension = file_name.split('.').next_back().unwrap_or("jpg");
        let generated_unique_name = format!(
            "{}/parts/{}_{}.{}", wo_id, Uuid::new_v4(), chrono::Utc::now().timestamp(), file_extension
        );
        let oci_object_name = oci::upload_object(&generated_unique_name, bytes, &content_type).await?;
        uploaded_object_names.push(oci_object_name);
    }

    // Prepare the effect using the service layer
    let payload = crate::model::requests::inventory::add_parts_request::AddPartsRequest {
        part_number,
        part_types_id,
        model_code,
        serial_number,
        description,
        photos: uploaded_object_names,
    };

    let effect = crate::services::v1::inventory::add_parts::decide_add_parts(payload, wo, auth.user.id)?;

    // Persist to database
    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.part_form_model.insert(txn).await?;
        for image_model in effect.image_models { image_model.insert(txn).await?; }
        for link_model in effect.image_link_models { link_model.insert(txn).await?; }
        Ok(())
    })).await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    // ── Notify SuperAdmins and province Admins about new parts ──
    let cfg = AppConfig::get();
    let system_user_id = cfg.system_user_id;
    let super_admin_role_id = luts.roles_by_name.get("SuperAdmin").copied();
    let admin_role_id = luts.roles_by_name.get("Admin").copied();

    let notification_data = serde_json::json!({
        "workOrderId": wo_id,
        "workOrderNumber": wo_number,
        "technicianName": auth.user.full_name,
        "province": wo_province,
    });

    let title = format!("New Parts Added: {}", wo_number);
    let body = format!(
        "Technician {} added new parts to {} in {}",
        auth.user.full_name, wo_number, wo_province
    );

    // Notify SuperAdmins
    if let Some(sa_role_id) = super_admin_role_id {
        if let Ok(super_admins) = users::Entity::find()
            .filter(users::Column::RoleId.eq(sa_role_id))
            .filter(users::Column::DeletedAt.is_null())
            .all(db.as_ref())
            .await
        {
            for sa in super_admins {
                if sa.id == system_user_id { continue; }
                let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                    mongodb.as_ref(),
                    valkey_client.clone(),
                    db.as_ref(),
                    sa.id,
                    "add_new_part",
                    &title,
                    &body,
                    notification_data.clone(),
                ).await;
            }
        }
    }

    // Notify province Admins
    if let Some(a_role_id) = admin_role_id {
        if let Ok(province_admins) = users::Entity::find()
            .filter(users::Column::RoleId.eq(a_role_id))
            .filter(users::Column::Province.eq(&wo_province))
            .filter(users::Column::DeletedAt.is_null())
            .all(db.as_ref())
            .await
        {
            for admin in province_admins {
                if admin.id == system_user_id { continue; }
                let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                    mongodb.as_ref(),
                    valkey_client.clone(),
                    db.as_ref(),
                    admin.id,
                    "add_new_part",
                    &title,
                    &body,
                    notification_data.clone(),
                ).await;
            }
        }
    }

    Ok(Json(ApiResponse::message_only(200, "Parts added successfully")))
}
