use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::model::requests::inventory::verify_product_request::VerifyProductRequest;
use crate::model::responses::inventory::verify_product_response::VerifyProductResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::verify_product::determine_verify_product_result;
use crate::entities::{registered_devices, warranties};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use chrono::Utc;
use validator::Validate;

/// Verify a product by serial number and check if it has been registered.
///
/// This endpoint checks if a product exists in the SCM catalog by serial number,
/// whether it has already been registered by any customer, and if the warranty is still valid.
/// Products with expired or no warranty cannot be verified.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/products/verify",
    tag = "inventory",
    request_body = VerifyProductRequest,
    responses(
        (status = 200, description = "Product verification completed", body = ApiResponse<VerifyProductResponse>),
        (status = 400, description = "Bad Request - Product warranty is expired or not available", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn verify_product(
    State(state): State<AppState>,
    Json(payload): Json<VerifyProductRequest>,
) -> Result<Json<ApiResponse<VerifyProductResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Find product in Zeus SCM by serial number
    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?
        .ok_or_else(|| AppError::NotFound("Product not found in catalog".to_string()))?;

    // Check if device is already registered by any customer
    let existing_registration = registered_devices::Entity::find()
        .filter(registered_devices::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?;

    let is_registered = existing_registration.is_some();

    // Check warranty status
    let existing_warranty = warranties::Entity::find()
        .filter(warranties::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?;

    let res = determine_verify_product_result(
        zeus_prod.id,
        &zeus_prod.serial_number,
        &zeus_prod.product_name,
        &zeus_prod.product_model_code,
        is_registered,
        existing_warranty,
        Utc::now(),
    );

    // Reject if warranty is expired or not available
    if res.warranty_status != "active" {
        return Err(AppError::BadRequest(
            "Product cannot be verified: warranty is expired or not available".to_string()
        ));
    }

    Ok(Json(ApiResponse::success(
        200,
        "Product verification completed",
        res,
    )))
}
