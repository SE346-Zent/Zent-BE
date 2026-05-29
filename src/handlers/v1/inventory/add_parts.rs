use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::config::AppConfig;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::inventory::add_parts_request::AddPartsRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::{work_orders as work_orders_ent, users};

#[utoipa::path(
    post, path = "/api/v1/inventory/work_orders/{id}/parts", request_body = AddPartsRequest,
    tag = "inventory",
    responses(
        (status = 200, description = "Parts added successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"), (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
/// Handle requests from technicians to register new parts against a specific work order.
///
/// This handler verifies the work order exists, validates that the requesting 
/// technician is assigned to it, and performs a multi-table database transaction
/// to persist the part registration form and any associated photo records.
///
/// # Arguments
/// * `authenticated_user` - The currently authenticated user (must be the assigned technician).
/// * `db_connection` - Shared database connection pool.
/// * `work_order_id` - The unique ID of the work order to which parts are being added.
/// * `add_parts_payload` - The request containing part metadata and photo filenames.
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
    Json(payload): Json<AddPartsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    // Capture fields before wo is moved into the service effect
    let wo_id = wo.id;
    let wo_number = wo.work_order_number.clone();
    let wo_province = wo.province.clone();

    let effect = crate::services::v1::inventory::add_parts::decide_add_parts(payload, wo, auth.user.id)?;

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

    let title = format!("New Parts Added: Work Order {}", wo_number);
    let body = format!(
        "Technician {} added new parts to WO {} in {}",
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
