use axum::{
    extract::{Path, State},
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use std::sync::Arc;
use std::collections::HashMap;
use validator::Validate;
use uuid::Uuid;

use crate::{
    core::errors::{AppError, ErrorResponse},
    core::lookup_tables::LookupTables,
    core::state::AppState,
    entities::{warranties, work_orders as work_orders_ent, users},
    extractor::auth_user::AuthUser,
    infrastructure::cache::ValkeyClient,
    model::requests::work_orders::edit_request::EditWorkOrderRequest,
    model::responses::base::ApiResponse,
    services::v1::work_orders::edit as edit_svc,
    services::v1::work_orders::auto_assign,
};

/// Allow the customer who owns a work order to edit selected fields while it is still
/// `Pending` or `Assigned`, and only up to the configured number of hours before the
/// scheduled appointment.
///
/// After a successful edit, the system re-runs the auto-assign algorithm to find the
/// best technician for the (possibly changed) appointment and location. If the same
/// technician is selected, no notifications are sent. If a different technician is
/// selected, all stakeholders (customer, old technician, new technician) are notified.
#[utoipa::path(
    post,
    path = "/api/v1/work_orders/{workOrderNumber}/edit",
    request_body = EditWorkOrderRequest,
    responses(
        (status = 200, description = "Work order updated successfully", body = MessageOnlyResponseBody),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 422, description = "Warranty / business validation error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn edit(
    Extension(auth): Extension<AuthUser>,
    State(state): State<AppState>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(work_order_number): Path<String>,
    Json(payload): Json<EditWorkOrderRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Customers reference a work order by its business number, not its UUID.
    let work_order = super::get_work_order_by_number(db.as_ref(), &work_order_number).await?;

    let pending_status_id = *luts
        .work_order_statuses_by_name
        .get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;
    let assigned_status_id = *luts
        .work_order_statuses_by_name
        .get("Assigned")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Assigned' status missing")))?;

    let edit_window_hours: i64 = luts
        .policies
        .get("customer_edit_window_hours")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    // Determine whether the customer is changing the product and, if so,
    // look up the warranty for the new product.
    let new_product_id = payload.product_id;
    let product_id_changed = new_product_id
        .map(|p| p != work_order.product_id)
        .unwrap_or(false);

    let new_product_warranty = if product_id_changed {
        let new_pid = new_product_id.expect("checked above");
        let _ = state.zeus_client.get_product(new_pid).await?;

        warranties::Entity::find()
            .filter(warranties::Column::ProductId.eq(new_pid))
            .one(db.as_ref())
            .await?
    } else {
        None
    };

    let ctx = edit_svc::EditWorkOrderContext {
        new_product_id,
        product_id_changed,
        new_product_warranty,
        now: chrono::Utc::now(),
    };

    let effect = edit_svc::decide_edit_work_order(
        work_order.clone(),
        auth.user.id,
        pending_status_id,
        assigned_status_id,
        edit_window_hours,
        payload,
        ctx,
    )?;

    let old_technician_id = effect.old_technician_id;
    let wo_id = work_order.id;
    let wo_number = work_order.work_order_number.clone();
    let customer_id = work_order.customer_id;

    // Save the edit changes in a transaction
    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order_model.update(txn).await?;
        Ok(())
    }))
    .await
    .map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Re-fetch the updated work order from DB for auto-assign
    let updated_wo = work_orders_ent::Entity::find_by_id(wo_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found after edit".to_string()))?;

    // Run auto-assign algorithm on the updated work order
    let assigned = run_auto_assign_after_edit(
        &state,
        db.clone(),
        &luts,
        updated_wo.clone(),
    ).await;

    // Handle notifications based on assignment result
    match assigned {
        Some(new_tech_id) if Some(new_tech_id) != old_technician_id => {
            // New technician assigned — notify all stakeholders
            let customer = users::Entity::find_by_id(customer_id)
                .one(db.as_ref())
                .await.unwrap_or_default();
            let new_tech = users::Entity::find_by_id(new_tech_id)
                .one(db.as_ref())
                .await.unwrap_or_default();

            let notification_data = serde_json::json!({
                "workOrderId": wo_id,
                "workOrderNumber": wo_number,
                "appointment": updated_wo.appointment.to_string(),
            });

            // Notify customer about reassignment
            if let Some(ref c) = customer {
                let new_tech_name = new_tech.as_ref().map(|t| t.full_name.as_str()).unwrap_or("a technician");
                let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                    state.mongodb.as_ref(),
                    state.valkey.clone(),
                    db.as_ref(),
                    c.id,
                    "work_order_assigned",
                    "Work Order Reassigned",
                    &format!("Your work order {} has been reassigned to technician {}", wo_number, new_tech_name),
                    notification_data.clone(),
                ).await;
            }

            // Notify new technician
            if let Some(ref t) = new_tech {
                let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                    state.mongodb.as_ref(),
                    state.valkey.clone(),
                    db.as_ref(),
                    t.id,
                    "work_order_assigned",
                    "New Work Order Assigned",
                    &format!("You have been assigned to work order {}", wo_number),
                    notification_data.clone(),
                ).await;
            }

            // Notify old technician about unassignment (if there was one)
            if let Some(old_tech_id) = old_technician_id {
                let old_tech = users::Entity::find_by_id(old_tech_id)
                    .one(db.as_ref())
                    .await.unwrap_or_default();
                if let Some(ref ot) = old_tech {
                    let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                        state.mongodb.as_ref(),
                        state.valkey.clone(),
                        db.as_ref(),
                        ot.id,
                        "work_order_assigned",
                        "Work Order Unassigned",
                        &format!("You have been unassigned from work order {} due to customer schedule changes", wo_number),
                        notification_data.clone(),
                    ).await;
                }

                // Clean up chat room if it exists (old tech-customer room)
                if let Some(_chat_room_id) = updated_wo.chat_room_id {
                    if let (Some(ref new_t), Some(ref c)) = (&new_tech, &customer) {
                        // Reassign the chat room to the new technician
                        let _ = crate::handlers::v1::work_orders::assign::ensure_chat_room(
                            db.as_ref(), new_t.id, c.id, wo_id,
                        ).await;
                    }
                } else if let (Some(ref new_t), Some(ref c)) = (&new_tech, &customer) {
                    let _ = crate::handlers::v1::work_orders::assign::ensure_chat_room(
                        db.as_ref(), new_t.id, c.id, wo_id,
                    ).await;
                }
            } else {
                // Was unassigned before, now assigned — create chat room
                if let (Some(ref t), Some(ref c)) = (&new_tech, &customer) {
                    let _ = crate::handlers::v1::work_orders::assign::ensure_chat_room(
                        db.as_ref(), t.id, c.id, wo_id,
                    ).await;
                }
            }

            // Send email notifications
            if let Some(rmq) = state.rabbitmq.as_ref() {
                if let (Some(ref c), Some(ref t)) = (&customer, &new_tech) {
                    let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(
                        rmq, &state.templates, &c.email, &c.full_name, &wo_number, &t.full_name, &updated_wo.appointment.to_string(),
                    ).await;
                }
            }
        }
        Some(_) => {
            // Same technician — no notifications needed
            tracing::info!("Auto-assign after edit for WO {} kept the same technician", wo_number);
        }
        None => {
            // No technician available — if was previously assigned, unassign
            if old_technician_id.is_some() {
                tracing::info!("Auto-assign after edit found no technician for WO {} — leaving unassigned", wo_number);
            }
        }
    }

    // Write-through cache: store full WorkOrderDetails in cache and bump list generation
    super::write_through_work_order_cache(
        db.as_ref(),
        valkey_client,
        luts.as_ref(),
        wo_id,
    )
    .await;

    Ok(Json(ApiResponse::message_only(200, "Work order updated successfully")))
}

/// Run the auto-assign algorithm on an updated work order and persist the result.
///
/// Returns the ID of the assigned technician, or `None` if no suitable technician
/// was found.
pub(super) async fn run_auto_assign_after_edit(
    _state: &AppState,
    db: Arc<DatabaseConnection>,
    luts: &LookupTables,
    wo: work_orders_ent::Model,
) -> Option<Uuid> {
    let tech_role_id = match luts.roles_by_name.get("Technician") {
        Some(id) => *id,
        None => {
            tracing::warn!("Technician role not found");
            return None;
        }
    };
    let policies = &luts.policies;
    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned").unwrap();
    let done_status_id = *luts.work_order_statuses_by_name.get("Closed").unwrap_or_else(|| luts.work_order_statuses_by_name.get("Pending").unwrap());
    let cfg = crate::core::config::AppConfig::get();
    let system_user_id = cfg.system_user_id;
    let province = wo.province.clone();

    let technicians = match users::Entity::find()
        .filter(users::Column::RoleId.eq(tech_role_id))
        .filter(users::Column::Province.eq(&province))
        .all(db.as_ref())
        .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("Failed to find technicians: {}", e);
            return None;
        }
    };

    if technicians.is_empty() {
        tracing::info!("Auto-assign after edit: no technicians in province '{}' for WO {}", province, wo.work_order_number);
        return None;
    }

    let tech_ids: Vec<Uuid> = technicians.iter().map(|t| t.id).collect();
    let agendas = match work_orders_ent::Entity::find()
        .filter(work_orders_ent::Column::DeletedAt.is_null())
        .filter(work_orders_ent::Column::TechnicianId.is_in(tech_ids))
        .filter(work_orders_ent::Column::WorkOrderStatusId.ne(done_status_id))
        .all(db.as_ref())
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("Failed to load agendas: {}", e);
            return None;
        }
    };

    let mut technician_agendas: HashMap<Uuid, Vec<work_orders_ent::Model>> = HashMap::new();
    for a in agendas {
        if let Some(tid) = a.technician_id {
            technician_agendas.entry(tid).or_default().push(a);
        }
    }

    let effect = match auto_assign::decide_auto_assign(
        wo.clone(),
        technicians,
        technician_agendas,
        policies,
        assigned_status_id,
        done_status_id,
        system_user_id,
    ) {
        Ok(Some(eff)) => eff,
        Ok(None) => {
            tracing::info!("Auto-assign after edit: no suitable tech for WO {}", wo.work_order_number);
            return None;
        }
        Err(e) => {
            tracing::error!("Auto-assign after edit failed for WO {}: {}", wo.work_order_number, e);
            return None;
        }
    };

    let assigned_tech_id = effect.work_order_model.technician_id.clone().unwrap().unwrap();

    if let Err(e) = db.transaction::<_, (), anyhow::Error>(|txn| Box::pin(async move {
        effect.work_order_model.update(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        effect.state_history_model.insert(txn).await.map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    })).await {
        tracing::error!("Auto-assign tx after edit failed for WO {}: {}", wo.work_order_number, e);
        return None;
    }

    tracing::info!("Auto-assigned WO {} to {} after customer edit", wo.work_order_number, assigned_tech_id);
    Some(assigned_tech_id)
}

/// Lightweight schema alias so utoipa can describe the success body.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MessageOnlyResponseBody {
    #[schema(example = 200)]
    pub status_code: u16,
    #[schema(example = "Work order updated successfully")]
    pub message: String,
}
