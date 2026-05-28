use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::model::requests::inventory::check_warranty_request::CheckWarrantyRequest;
use crate::model::responses::inventory::warranty_check_response::WarrantyCheckResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::check_warranty::determine_warranty_status;
use crate::entities::warranties;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use chrono::Utc;
use validator::Validate;

/// Check warranty status of a product by serial number.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/products/check-warranty",
    request_body = CheckWarrantyRequest,
    responses(
        (status = 200, description = "Warranty information retrieved successfully", body = ApiResponse<WarrantyCheckResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn check_warranty(
    State(state): State<AppState>,
    Json(payload): Json<CheckWarrantyRequest>,
) -> Result<Json<ApiResponse<WarrantyCheckResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?
        .ok_or_else(|| AppError::BadRequest(format!("Serial number '{}' not found in product catalog", payload.serial_number)))?;

    let existing_warranty = warranties::Entity::find()
        .filter(warranties::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?;

    let res = determine_warranty_status(
        zeus_prod.id,
        &zeus_prod.serial_number,
        &zeus_prod.product_name,
        existing_warranty,
        Utc::now(),
    );

    Ok(Json(ApiResponse::success(
        200,
        "Warranty checked successfully",
        res,
    )))
}
