use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::start_request::StartWorkOrderRequest;
use crate::model::responses::base::ApiResponse;

/// Start work on a work order, performing geofencing validation to ensure technician presence.

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/start", request_body = StartWorkOrderRequest,
    responses(
        (status = 200, description = "Work order started successfully", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"), (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn start(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<StartWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;
    // Write-through: use the cache for individual work order instead of querying DB
    let wo = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;
    let in_prog_id = *luts.work_order_statuses_by_name.get("InProg").ok_or_else(|| AppError::Internal(anyhow::anyhow!("In Progress status missing from lookup tables")))?;

    let target_location = crate::utils::geocoding::geocode_address(
        &wo.address,
        &wo.ward,
        &wo.province,
        &wo.country,
    ).await?;

    let effect = crate::services::v1::work_orders::start::decide_start(payload, wo, auth.user.id, in_prog_id, &luts.policies, target_location.lat, target_location.lng).await?;
    db.transaction::<_, (), AppError>(|txn| Box::pin(async move { effect.work_order_model.update(txn).await?; effect.state_history_model.insert(txn).await?; Ok(()) }))
        .await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    super::track_wo_transition("Assigned", "InProg");

    Ok(Json(ApiResponse::message_only(200, "Work order started successfully")))
}
