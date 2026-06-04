use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, TransactionTrait, EntityTrait, Set};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::entities::parts as parts_entity;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::infrastructure::clients::zeus::ZeusClient;
use crate::model::requests::work_orders::complete_request::CompleteWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::ports::ZeusInventoryClient;

/// Complete a work order by submitting a closing form, including part changes and customer signature.

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/complete",
    request_body(content = CompleteWorkOrderRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Work order completed successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse), (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse), (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn complete(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CompleteWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let part_changes = payload.part_changes.clone();
    // Write-through: use the cache for individual work order instead of querying DB
    let wo = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;
    let work_order_product_id = wo.product_id;
    let wo_technician_id = wo.technician_id;

    let zeus_client = load_zeus_client()?;

    if wo.technician_id != Some(auth.user.id) { return Err(AppError::Forbidden("You are not assigned to this work order".to_string())); }

    let target = crate::utils::geocoding::geocode_address(&wo.address, &wo.ward, &wo.province, &wo.country).await?;
    let radius: f64 = luts.policies.get("geofencing_radius").and_then(|v| v.parse().ok()).unwrap_or(2000.0);
    if !crate::utils::geo::is_within_geofence(payload.latitude, payload.longitude, target.lat, target.lng, radius) {
        return Err(AppError::Forbidden("You are too far from the work site to complete this work order".to_string()));
    }

    let completed_id = *luts.work_order_statuses_by_name.get("Closed").ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;
    let effect = crate::services::v1::work_orders::complete::decide_complete_work_order(payload, wo, completed_id, auth.user.id)?;

    for part_change in &part_changes {
        let result = match part_change.change_type.as_str() {
            "installed" => {
                zeus_client.install_part(part_change.part_id, work_order_product_id).await
            }
            "uninstalled" => {
                zeus_client.remove_part(part_change.part_id).await
            }
            other => return Err(AppError::BadRequest(format!(
                "Invalid part change type '{}' for part {}. Must be 'installed' or 'uninstalled'.",
                other, part_change.part_id
            ))),
        };

        if let Err(e) = result {
            let action = part_change.change_type.as_str();
            tracing::error!(
                part_id = %part_change.part_id,
                change_type = action,
                error = %e,
                "Part operation failed during work order completion"
            );
            return Err(match e {
                AppError::NotFound(_) => AppError::BadRequest(format!(
                    "Cannot {} part {}: the part was not found in the inventory system. \
                     Verify the part ID is correct and the part exists.",
                    action, part_change.part_id
                )),
                AppError::BadRequest(msg) => AppError::BadRequest(format!(
                    "Cannot {} part {}: {}",
                    action, part_change.part_id, msg
                )),
                other => AppError::Internal(anyhow::anyhow!(
                    "Failed to {} part {} in inventory system: {}",
                    action, part_change.part_id, other
                )),
            });
        }
    }

    // Sync parts from SCM to local DB before inserting part_changes.
    // part_changes has a FK to parts(id) in Zent-BE's local MySQL, but parts
    // are managed in SCM's separate database. We must ensure each part row
    // exists locally before the transaction can succeed.
    for pc in &part_changes {
        let local_exists = parts_entity::Entity::find_by_id(pc.part_id)
            .one(db.as_ref())
            .await?
            .is_some();

        if !local_exists {
            match zeus_client.get_part(pc.part_id).await {
                Ok(scm_part) => {
                    let local_part = parts_entity::ActiveModel {
                        id: Set(scm_part.id),
                        part_catalog_id: Set(scm_part.part_catalog_id),
                        product_id: Set(scm_part.product_id),
                        serial_number: Set(scm_part.serial_number),
                        part_condition_id: Set(scm_part.part_condition_id),
                        manufactured_date: Set(scm_part.manufactured_date),
                        installation_date: Set(scm_part.installation_date),
                        removal_date: Set(scm_part.removal_date),
                        scrapped_date: Set(scm_part.scrapped_date),
                        created_at: Set(scm_part.created_at),
                        updated_at: Set(scm_part.updated_at),
                        ..Default::default()
                    };
                    local_part.insert(db.as_ref()).await?;
                    tracing::info!(
                        part_id = %pc.part_id,
                        "Synced part from SCM to local DB for work order completion"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        part_id = %pc.part_id,
                        error = %e,
                        "Failed to fetch part from SCM for local sync"
                    );
                    return Err(AppError::BadRequest(format!(
                        "Cannot process part {}: the part could not be found in the inventory system. \
                         Verify the part ID is correct.",
                        pc.part_id
                    )));
                }
            }
        }
    }

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.closing_form_model.insert(txn).await?;
        for img in effect.image_models { img.insert(txn).await?; }
        for link in effect.image_link_models { link.insert(txn).await?; }
        for pc in effect.part_change_models { pc.insert(txn).await?; }
        for pu in effect.part_record_updates { pu.update(txn).await?; }
        effect.work_order_model.update(txn).await?;
        effect.state_history_model.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    if let Some(bytes) = effect.checklist_json {
        let cfg = crate::core::config::AppConfig::get();
        let dir = format!("{}/{}", cfg.checklist_save_path, effect.closing_form_id);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create checklist dir: {}", e)))?;
        tokio::fs::write(format!("{}/checklist.json", dir), &bytes).await.map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to write checklist: {}", e)))?;
    }

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), id).await;

    // Decrement technician workload cache (work order moved to Closed)
    if let Some(tech_id) = wo_technician_id {
        super::decrement_technician_workload(&valkey_client, tech_id).await;
    }

    super::track_wo_transition("InProg", "Closed");

    Ok(Json(ApiResponse::success(200, "Work order completed successfully", ())))
}

fn load_zeus_client() -> Result<ZeusClient, AppError> {
    let base_url = std::env::var("ZEUS_BASE_URL")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ZEUS_BASE_URL is required for work order completion")))?;
    let api_key = std::env::var("ZEUS_API_KEY")
        .map_err(|_| AppError::Internal(anyhow::anyhow!("ZEUS_API_KEY is required for work order completion")))?;

    Ok(ZeusClient::new(base_url, api_key))
}
