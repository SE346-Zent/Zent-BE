use axum::{extract::{State, Path}, Json, Extension};
use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, TransactionTrait};
use uuid::Uuid;
use validator::Validate;
use crate::core::errors::AppError;
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::add_parts_request::AddPartsRequest;
use crate::model::responses::base::ApiResponse;
use crate::entities::work_orders as work_orders_ent;

#[utoipa::path(
    post, path = "/api/v1/inventory/work_orders/{id}/parts", request_body = AddPartsRequest,
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
    Extension(authenticated_user): Extension<AuthUser>,
    State(db_connection): State<Arc<DatabaseConnection>>,
    Path(work_order_id): Path<Uuid>,
    Json(add_parts_payload): Json<AddPartsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    add_parts_payload.validate().map_err(|err| AppError::BadRequest(err.to_string()))?;
    
    let work_order_record = work_orders_ent::Entity::find_by_id(work_order_id)
        .one(db_connection.as_ref()).await?
        .ok_or_else(|| AppError::NotFound("Work order not found".to_string()))?;
        
    let add_parts_effect = crate::services::v1::inventory::add_parts::decide_add_parts(
        add_parts_payload, 
        work_order_record, 
        authenticated_user.user.id
    )?;

    db_connection.transaction::<_, (), AppError>(|txn| Box::pin(async move {
        add_parts_effect.part_form_model.insert(txn).await?;
        for image_model in add_parts_effect.image_models { image_model.insert(txn).await?; }
        for link_model in add_parts_effect.image_link_models { link_model.insert(txn).await?; }
        Ok(())
    })).await.map_err(|err| match err { 
        sea_orm::TransactionError::Connection(e) => AppError::Internal(anyhow::anyhow!("DB Connection Error: {}", e)), 
        sea_orm::TransactionError::Transaction(e) => e 
    })?;

    Ok(Json(ApiResponse::message_only(200, "Parts added successfully")))
}
