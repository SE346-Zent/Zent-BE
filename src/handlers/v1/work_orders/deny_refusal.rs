use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::responses::base::ApiResponse;
use crate::entities::users;

/// Deny a technician's refusal, resetting the work order to 'Pending' for reassignment.

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/refusal/deny",
    responses(
        (status = 200, description = "Refusal denied, status reverted", body = ApiResponse<String>),
        (status = 400, description = "Bad Request"), (status = 403, description = "Forbidden"),
        (status = 404, description = "Work order not found"), (status = 500, description = "Internal Server Error")
    ),
    security(("bearer_auth" = []))
)]
pub async fn deny_refusal(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_opt): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // Write-through: use the cache for individual work order instead of querying DB
    let wo = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    // Province check: only regular Admins are province-scoped; SuperAdmin can manage any
    let admin_role_id = *luts.roles_by_name.get("Admin")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Admin role missing from lookup tables")))?;
    if auth.user.role_id == admin_role_id {
        let p = auth.user.province.as_ref().ok_or_else(|| AppError::Forbidden("Admin has no province assigned".to_string()))?;
        if p != &wo.province { return Err(AppError::Forbidden("Admin province does not match work order province".to_string())); }
    }
    let rf_id = wo.reject_form_id.ok_or_else(|| AppError::BadRequest("No rejection form".to_string()))?;
    let rf = crate::entities::work_order_reject_forms::Entity::find_by_id(rf_id).one(db.as_ref()).await?.ok_or_else(|| AppError::NotFound("Rejection form not found".to_string()))?;
    let pending_id = *luts.work_order_statuses_by_name.get("Pending").ok_or_else(|| AppError::Internal(anyhow::anyhow!("Pending status not found")))?;

    let customer_id = wo.customer_id;
    let wo_number = wo.work_order_number.clone();
    let effect = crate::services::v1::work_orders::deny_refusal::decide_deny_refusal(wo, rf, auth.user.id, pending_id)?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move { effect.work_order_model.update(txn).await?; effect.reject_form_model.update(txn).await?; effect.state_history_model.insert(txn).await?; Ok(()) }))
        .await.map_err(|e| match e { sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)), sea_orm::TransactionError::Transaction(e) => e })?;

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    if let Some(rmq) = rabbitmq_opt.as_ref() {
        if let Some(cust) = users::Entity::find_by_id(customer_id).one(db.as_ref()).await? {
            let _ = crate::services::v1::core::email_service::send_work_order_refusal_denied_email(rmq, &templates, &cust.email, &cust.full_name, &wo_number).await;
        }
    }
    Ok(Json(ApiResponse::message_only(200, "Refusal denied. Work order reset to Pending for reassignment.")))
}
