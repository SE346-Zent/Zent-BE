use axum::{
    extract::{Path, State},
    Extension, Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait};
use std::sync::Arc;
use validator::Validate;

use crate::{
    core::errors::{AppError, ErrorResponse},
    core::lookup_tables::LookupTables,
    core::state::AppState,
    entities::{warranties, work_orders as work_orders_ent, work_order_symptoms},
    extractor::auth_user::AuthUser,
    infrastructure::cache::ValkeyClient,
    model::requests::work_orders::edit_request::EditWorkOrderRequest,
    model::responses::base::ApiResponse,
    services::v1::work_orders::edit as edit_svc,
};

/// Allow the customer who owns a work order to edit selected fields while it is still
/// `Pending` or `Assigned`, and only up to the configured number of hours before the
/// scheduled appointment.
///
/// The work order is referenced by its business-friendly `work_order_number`
/// (e.g. `WO-AB12CD`) in the URL path. When the customer is changing the
/// `product_id`, the new product must be covered by an active warranty —
/// otherwise the request is rejected with a clear, user-friendly message
/// (HTTP 422) that the FE can surface directly.
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
    // look up the warranty for the new product. The Zeus client also enforces
    // that the new product exists.
    let new_product_id = payload.product_id;
    let product_id_changed = new_product_id
        .map(|p| p != work_order.product_id)
        .unwrap_or(false);

    let new_product_warranty = if product_id_changed {
        let new_pid = new_product_id.expect("checked above");

        // Ensure the new product exists in the catalog (returns 404 on miss)
        let _ = state.zeus_client.get_product(new_pid).await?;

        warranties::Entity::find()
            .filter(warranties::Column::ProductId.eq(new_pid))
            .one(db.as_ref())
            .await?
    } else {
        None
    };

    // If the customer is changing the symptom, verify the symptom ID exists.
    if let Some(symptom_id) = payload.work_order_symptom_id {
        let exists = work_order_symptoms::Entity::find_by_id(symptom_id)
            .one(db.as_ref())
            .await?
            .is_some();
        if !exists {
            return Err(AppError::BadRequest(format!(
                "Symptom with id {} does not exist",
                symptom_id
            )));
        }
    }

    // If the customer is changing the reference ticket, verify the referenced
    // work order exists and belongs to the same customer.
    if let Some(ref_id) = payload.reference_ticket_id {
        let referenced = work_orders_ent::Entity::find_by_id(ref_id)
            .filter(work_orders_ent::Column::DeletedAt.is_null())
            .one(db.as_ref())
            .await?
            .ok_or_else(|| AppError::BadRequest("Reference work order not found".to_string()))?;
        if referenced.customer_id != auth.user.id {
            return Err(AppError::BadRequest(
                "Reference work order does not belong to you".to_string(),
            ));
        }
    }

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

    db.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        effect.work_order_model.update(txn).await?;
        Ok(())
    }))
    .await
    .map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Error: {}", e)),
        sea_orm::TransactionError::Transaction(e) => e,
    })?;

    // The cache is keyed on the work order UUID, so we must invalidate it there
    // after mutating the record — the next read will repopulate it.
    super::write_through_work_order_cache(
        db.as_ref(),
        valkey_client,
        luts.as_ref(),
        work_order.id,
    )
    .await;

    Ok(Json(ApiResponse::message_only(200, "Work order updated successfully")))
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
