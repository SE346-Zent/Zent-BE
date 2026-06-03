use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};

use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::AppState;
use crate::entities::{warranties, warranty_statuses};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::create_warranty_request::CreateWarrantyRequest;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::warranty_check_response::WarrantyCheckResponse;
use validator::Validate;

/// Create a warranty for a product identified by its serial number.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/warranties",
    tag = "inventory",
    request_body = CreateWarrantyRequest,
    responses(
        (status = 201, description = "Warranty created successfully", body = ApiResponse<WarrantyCheckResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 409, description = "Warranty already exists for this product", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_warranty(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateWarrantyRequest>,
) -> Result<Json<ApiResponse<WarrantyCheckResponse>>, AppError> {
    // Only admin can create warranties
    match auth.role.name.as_str() {
        "Admin" | "SuperAdmin" => {}
        _ => return Err(AppError::Forbidden("Only administrators can create warranties".to_string())),
    }

    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    if payload.end_date <= payload.start_date {
        return Err(AppError::BadRequest("End date must be after start date".to_string()));
    }

    // Look up product in SCM by serial number
    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?
        .ok_or_else(|| AppError::NotFound("Product not found in catalog".to_string()))?;

    // Check if warranty already exists
    let existing = warranties::Entity::find()
        .filter(warranties::Column::ProductId.eq(zeus_prod.id))
        .filter(warranties::Column::DeletedAt.is_null())
        .one(state.db.as_ref())
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("A warranty already exists for this product".to_string()));
    }

    // Derive warranty status
    let now = Utc::now();
    let warranty_status = if now > payload.end_date {
        "expired".to_string()
    } else {
        "active".to_string()
    };

    // Look up warranty_status_id from LUT
    let warranty_status_id = state.lookup_tables
        .warranty_statuses_by_name
        .get(&warranty_status)
        .copied();

    let warranty_id = uuid::Uuid::new_v4();

    let model = warranties::ActiveModel {
        id: Set(warranty_id),
        customer_id: Set(zeus_prod.customer_id),
        product_id: Set(zeus_prod.id),
        start_date: Set(payload.start_date),
        end_date: Set(payload.end_date),
        warranty_status: Set(warranty_status.clone()),
        warranty_status_id: Set(warranty_status_id),
        created_at: Set(now),
        updated_at: Set(now),
        deleted_at: Set(None),
    };

    model.insert(state.db.as_ref()).await?;

    let days_remaining = if warranty_status == "expired" {
        0
    } else {
        (payload.end_date - now).num_days().max(0)
    };

    Ok(Json(ApiResponse::success(
        201,
        "Warranty created successfully",
        WarrantyCheckResponse {
            product_id: zeus_prod.id,
            serial_number: zeus_prod.serial_number,
            product_name: zeus_prod.product_name,
            warranty_status: if days_remaining > 0 {
                format!("{} days remaining", days_remaining)
            } else {
                warranty_status
            },
            start_date: Some(payload.start_date.to_rfc3339()),
            end_date: Some(payload.end_date.to_rfc3339()),
        },
    )))
}
