use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use sea_orm::{QueryFilter, ColumnTrait};
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::reassign_request::ReassignWorkOrderRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::{work_orders as work_orders_ent, users};

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/reassign", request_body = ReassignWorkOrderRequest,
    responses(
        (status = 200, description = "Work order reassigned successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn reassign(
    Extension(auth): Extension<AuthUser>,
    State(db): State<Arc<DatabaseConnection>>,
    State(mongodb): State<Arc<mongodb::Database>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    State(rabbitmq_opt): State<Option<Arc<lapin::Connection>>>,
    State(templates): State<Arc<std::collections::HashMap<String, String>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReassignWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    // Province check: only regular Admins are province-scoped; SuperAdmin can reassign anywhere
    let admin_role_id = *luts.roles_by_name.get("Admin")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Admin role missing from lookup tables")))?;
    if auth.user.role_id == admin_role_id {
        let p = auth.user.province.as_ref().ok_or_else(|| AppError::Forbidden("Admin has no province assigned".to_string()))?;
        if p != &work_order.province { return Err(AppError::Forbidden("Admin province does not match work order province".to_string())); }
    }

    // Remember the old technician for notifications
    let old_tech_id = work_order.technician_id;

    let technician_work_orders = work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::TechnicianId.eq(payload.technician_id))
        .all(db.as_ref()).await?;

    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Assigned' status missing")))?;
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Closed' status missing")))?;

    let effect = crate::services::v1::work_orders::reassign::decide_reassign_work_order(
        payload.clone(),
        work_order.clone(),
        technician_work_orders,
        &luts.policies,
        assigned_status_id,
        done_status_id,
        auth.user.id,
    )?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order_model.update(txn).await?;
        effect.state_history_model.insert(txn).await?;
        Ok(())
    }))
    .await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    super::write_through_work_order_cache(db.as_ref(), valkey_client.clone(), luts.as_ref(), id).await;

    let new_tech = users::Entity::find_by_id(payload.technician_id).one(db.as_ref()).await?;
    let old_tech = if let Some(otid) = old_tech_id {
        users::Entity::find_by_id(otid).one(db.as_ref()).await?
    } else { None };
    let cust = users::Entity::find_by_id(work_order.customer_id).one(db.as_ref()).await?;

    // ── Notifications ──────────────────────────────────────────────
    let notification_data = serde_json::json!({
        "workOrderId": work_order.id,
        "workOrderNumber": work_order.work_order_number,
        "appointment": work_order.appointment.to_string(),
    });

    // Notify the new technician
    if let Some(t) = new_tech.as_ref() {
        let mut data = notification_data.clone();
        data["technicianName"] = serde_json::Value::String(t.full_name.clone());
        let _ = crate::handlers::v1::notifications::send_notification::send_notification(
            mongodb.as_ref(),
            valkey_client.clone(),
            db.as_ref(),
            t.id,
            "work_order_assigned",
            "Work Order Reassigned to You",
            &format!("You have been reassigned to work order {}", work_order.work_order_number),
            data,
        ).await;
    }

    // Notify the old technician (unassigned)
    if let Some(t) = old_tech.as_ref() {
        let mut data = notification_data.clone();
        data["technicianName"] = serde_json::Value::String(t.full_name.clone());
        let _ = crate::handlers::v1::notifications::send_notification::send_notification(
            mongodb.as_ref(),
            valkey_client.clone(),
            db.as_ref(),
            t.id,
            "work_order_assigned",
            "Work Order Unassigned",
            &format!("You have been unassigned from work order {}", work_order.work_order_number),
            data,
        ).await;
    }

    // Notify the customer
    if let (Some(nt), Some(c)) = (new_tech.as_ref(), cust.as_ref()) {
        let mut data = notification_data;
        data["technicianName"] = serde_json::Value::String(nt.full_name.clone());
        let _ = crate::handlers::v1::notifications::send_notification::send_notification(
            mongodb.as_ref(),
            valkey_client.clone(),
            db.as_ref(),
            c.id,
            "work_order_assigned",
            "Technician Changed",
            &format!("Technician for your work order {} has been changed to {}", work_order.work_order_number, nt.full_name),
            data,
        ).await;
    }

    // Send email notification to customer about reassignment
    if let Some(rmq) = rabbitmq_opt.as_ref() {
        if let (Some(nt), Some(c)) = (new_tech, cust) {
            let _ = crate::services::v1::core::email_service::send_work_order_reassigned_email(
                rmq, &templates, &c.email, &c.full_name,
                &work_order.work_order_number, &nt.full_name,
                &work_order.appointment.to_string(),
            ).await;
        }
    }

    Ok(Json(ApiResponse::success(200, "Work order reassigned successfully", ())))
}
