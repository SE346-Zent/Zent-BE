use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::work_orders::complete_request::CompleteWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::work_orders as work_orders_ent;

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
    Path(id): Path<Uuid>,
    Json(payload): Json<CompleteWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    let wo = work_orders_ent::Entity::find_by_id(id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;

    if wo.technician_id != Some(auth.user.id) { return Err(AppError::Forbidden("You are not assigned to this work order".to_string())); }

    let target = crate::utils::geocoding::geocode_address(&wo.address, &wo.city, &wo.province, &wo.country).await?;
    let radius: f64 = luts.policies.get("geofencing_radius").and_then(|v| v.parse().ok()).unwrap_or(2000.0);
    if !crate::utils::geo::is_within_geofence(payload.latitude, payload.longitude, target.lat, target.lng, radius) {
        return Err(AppError::Forbidden("Geofencing violation: You are too far from the work site".to_string()));
    }

    let completed_id = *luts.work_order_statuses_by_name.get("Closed").ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;
    let effect = crate::services::v1::work_orders::complete::decide_complete_work_order(payload, wo, completed_id, auth.user.id)?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.closing_form.insert(txn).await?;
        for img in effect.images { img.insert(txn).await?; }
        for link in effect.image_links { link.insert(txn).await?; }
        for pc in effect.part_changes { pc.insert(txn).await?; }
        for pu in effect.part_updates { pu.update(txn).await?; }
        effect.work_order.update(txn).await?;
        effect.state_history.insert(txn).await?;
        Ok(())
    })).await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    if let Some(bytes) = effect.checklist_json {
        let dir = format!("zent_checklist/{}", effect.closing_form_id);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to create checklist dir: {}", e)))?;
        tokio::fs::write(format!("{}/checklist.json", dir), &bytes).await.map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to write checklist: {}", e)))?;
    }
    Ok(Json(ApiResponse::success(200, "Work order completed successfully", ())))
}
