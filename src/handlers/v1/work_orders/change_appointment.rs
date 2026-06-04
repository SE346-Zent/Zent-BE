use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, TransactionTrait, ActiveModelTrait, EntityTrait, QueryFilter, ColumnTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::AppState;
use crate::entities::{work_orders as work_orders_ent, users};
use crate::extractor::auth_user::AuthUser;
use crate::infrastructure::cache::ValkeyClient;
use crate::model::requests::work_orders::change_appointment_request::ChangeAppointmentRequest;
use crate::model::responses::base::ApiResponse;

#[utoipa::path(
    post, path = "/api/v1/work_orders/{id}/change-appointment", request_body = ChangeAppointmentRequest,
    responses(
        (status = 200, description = "Appointment changed successfully"),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not Found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn change_appointment(
    Extension(auth): Extension<AuthUser>,
    State(state): State<AppState>,
    State(db): State<Arc<DatabaseConnection>>,
    State(luts): State<Arc<LookupTables>>,
    State(valkey_client): State<Option<Arc<ValkeyClient>>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ChangeAppointmentRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let work_order = super::get_cached_work_order_model(db.as_ref(), &valkey_client, id).await?;

    // Province check for regular Admins (SuperAdmin bypasses)
    let admin_role_id = *luts.roles_by_name.get("Admin")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Admin role missing from lookup tables")))?;
    if auth.user.role_id == admin_role_id {
        let p = auth.user.province.as_ref().ok_or_else(|| AppError::Forbidden("Your admin profile does not have a province assigned".to_string()))?;
        if p != &work_order.province {
            return Err(AppError::Forbidden("You do not have permission to manage work orders in this province".to_string()));
        }
    }

    // Prevent scheduling conflicts: the technician cannot have two appointments
    // at the exact same time. Only enforced when a technician is assigned.
    if let Some(tech_id) = work_order.technician_id {
        let conflict = work_orders_ent::Entity::find()
            .filter(work_orders_ent::Column::DeletedAt.is_null())
            .filter(work_orders_ent::Column::TechnicianId.eq(tech_id))
            .filter(work_orders_ent::Column::Id.ne(work_order.id))
            .filter(work_orders_ent::Column::Appointment.eq(payload.new_appointment))
            .one(db.as_ref())
            .await?;
        if conflict.is_some() {
            return Err(AppError::Conflict(
                "Technician already has an appointment at that time".into(),
            ));
        }
    }

    let old_technician_id = work_order.technician_id;
    let wo_id = work_order.id;
    let wo_number = work_order.work_order_number.clone();
    let customer_id = work_order.customer_id;

    let pending_status_id = *luts.work_order_statuses_by_name.get("Pending")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Pending' status missing")))?;
    let assigned_status_id = *luts.work_order_statuses_by_name.get("Assigned")
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("'Assigned' status missing")))?;

    let effect = crate::services::v1::work_orders::change_appointment::decide_change_appointment(
        work_order,
        payload.new_appointment,
        pending_status_id,
        assigned_status_id,
        auth.user.id,
        &luts.policies,
    )?;

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order.update(txn).await?;
        effect.audit.insert(txn).await?;
        Ok(())
    }))
    .await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // Re-fetch the updated work order from DB for auto-assign
    let updated_wo = work_orders_ent::Entity::find_by_id(wo_id)
        .one(db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("Work order not found after appointment change".to_string()))?;

    // Run auto-assign algorithm on the updated work order
    let assigned = super::edit::run_auto_assign_after_edit(
        &state,
        db.clone(),
        &luts,
        updated_wo.clone(),
    ).await;

    // Handle notifications based on assignment result
    match assigned {
        Some(new_tech_id) if Some(new_tech_id) != old_technician_id => {
            let customer = users::Entity::find_by_id(customer_id)
                .one(db.as_ref()).await.unwrap_or_default();
            let new_tech = users::Entity::find_by_id(new_tech_id)
                .one(db.as_ref()).await.unwrap_or_default();

            let notification_data = serde_json::json!({
                "workOrderId": wo_id,
                "workOrderNumber": wo_number,
                "appointment": updated_wo.appointment.to_string(),
            });

            // Notify old technician about unassignment
            if let Some(old_tech_id) = old_technician_id {
                let old_tech = users::Entity::find_by_id(old_tech_id)
                    .one(db.as_ref()).await.unwrap_or_default();
                if let Some(ref ot) = old_tech {
                    let _ = crate::handlers::v1::notifications::send_notification::send_notification(
                        state.mongodb.as_ref(),
                        state.valkey.clone(),
                        db.as_ref(),
                        ot.id,
                        "work_order_assigned",
                        "Work Order Unassigned",
                        &format!("You have been unassigned from work order {} due to appointment change", wo_number),
                        notification_data.clone(),
                    ).await;
                }

                // Reassign or create chat room for new tech + customer
                if let (Some(ref new_t), Some(ref c)) = (&new_tech, &customer) {
                    let _ = crate::handlers::v1::work_orders::assign::ensure_chat_room(
                        db.as_ref(), new_t.id, c.id, wo_id,
                    ).await;
                }
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

            // Notify customer
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
                    notification_data,
                ).await;
            }

            // Send email notifications to new tech and customer
            if let Some(rmq) = state.rabbitmq.as_ref() {
                // Email to new technician
                if let Some(ref t) = new_tech {
                    let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(
                        rmq, &state.templates, &t.email, &t.full_name,
                        &wo_number, &t.full_name, &updated_wo.appointment.to_string(),
                    ).await;
                }
                // Email to customer
                if let (Some(ref c), Some(ref t)) = (&customer, &new_tech) {
                    let _ = crate::services::v1::core::email_service::send_work_order_assigned_email(
                        rmq, &state.templates, &c.email, &c.full_name,
                        &wo_number, &t.full_name, &updated_wo.appointment.to_string(),
                    ).await;
                }
            }
        }
        Some(_) => {
            tracing::info!("Auto-assign after appointment change for WO {} kept the same technician", wo_number);
        }
        None => {
            tracing::info!("Auto-assign after appointment change found no technician for WO {}", wo_number);
        }
    }

    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    Ok(Json(ApiResponse::message_only(200, "Appointment changed successfully")))
}
