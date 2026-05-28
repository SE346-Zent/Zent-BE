use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, TransactionTrait, ActiveModelTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::lookup_tables::LookupTables;
use crate::core::errors::{AppError, ErrorResponse};
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
        let p = auth.user.province.as_ref().ok_or_else(|| AppError::Forbidden("Admin has no province assigned".to_string()))?;
        if p != &work_order.province {
            return Err(AppError::Forbidden("Admin province does not match work order province".to_string()));
        }
    }

    // Prevent scheduling conflicts: the technician cannot have two appointments
    // at the exact same time. Only enforced when a technician is assigned.
    if let Some(tech_id) = work_order.technician_id {
        use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
        use crate::entities::work_orders as work_orders_ent;
        let conflict = work_orders_ent::Entity::find()
            .filter(work_orders_ent::Column::DeletedAt.is_null())
            .filter(work_orders_ent::Column::TechnicianId.eq(tech_id))
            .filter(work_orders_ent::Column::Id.ne(work_order.id))
            .filter(work_orders_ent::Column::Appointment.eq(payload.new_appointment))
            .one(db.as_ref())
            .await?;
        if conflict.is_some() {
            return Err(AppError::Conflict(
                "Technician already has an appointment at this time".into(),
            ));
        }
    }

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

    super::write_through_work_order_cache(db.as_ref(), valkey_client, luts.as_ref(), id).await;

    Ok(Json(ApiResponse::message_only(200, "Appointment changed successfully")))
}
