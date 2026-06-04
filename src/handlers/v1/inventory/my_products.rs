use axum::{extract::State, Json};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::AppState;
use crate::entities::{registered_devices, warranties};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::my_products_response::{
    MyProductItem, MyProductWarranty,
};

/// List products registered by the authenticated customer, enriched with model image and warranty.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/products/mine",
    tag = "inventory",
    responses(
        (status = 200, description = "Customer products retrieved successfully", body = ApiResponse<Vec<MyProductItem>>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn my_products(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<MyProductItem>>>, AppError> {
    // 1. Get all registered devices for this customer
    let registrations = registered_devices::Entity::find()
        .filter(registered_devices::Column::CustomerId.eq(auth.user.id))
        .filter(registered_devices::Column::DeletedAt.is_null())
        .all(state.db.as_ref())
        .await?;

    if registrations.is_empty() {
        return Ok(Json(ApiResponse::success(
            200,
            "No registered products found",
            Vec::<MyProductItem>::new(),
        )));
    }

    let product_ids: Vec<uuid::Uuid> = registrations.iter().map(|r| r.product_id).collect();

    // 2. Batch-fetch warranties for all registered products
    let warranty_records = warranties::Entity::find()
        .filter(warranties::Column::ProductId.is_in(product_ids.clone()))
        .filter(warranties::Column::DeletedAt.is_null())
        .all(state.db.as_ref())
        .await?;

    let warranty_map: std::collections::HashMap<uuid::Uuid, warranties::Model> =
        warranty_records.into_iter().map(|w| (w.product_id, w)).collect();

    // 3. For each registration, fetch product + model from SCM, build response
    let mut items = Vec::with_capacity(registrations.len());

    for reg in &registrations {
        let zeus_prod = match state.zeus_client.get_product(reg.product_id).await {
            Ok(p) => p,
            Err(_) => continue, // skip products no longer in SCM
        };

        // Fetch product model for image
        let image_url = match state
            .zeus_client
            .get_product_model(&zeus_prod.product_model_code)
            .await
        {
            Ok(model) => model.image_url,
            Err(_) => None,
        };

        // Build warranty response
        let now = Utc::now();
        let warranty = warranty_map.get(&reg.product_id).map(|w| {
            let is_expired = now > w.end_date;
            let is_voided = w.warranty_status.eq_ignore_ascii_case("voided");
            let days_remaining = if is_voided || is_expired {
                0
            } else {
                (w.end_date - now).num_days().max(0)
            };
            let status = if is_voided {
                "Voided".to_string()
            } else if is_expired {
                "Expired".to_string()
            } else {
                w.warranty_status.clone()
            };

            MyProductWarranty {
                id: w.id,
                start_date: w.start_date.to_rfc3339(),
                end_date: w.end_date.to_rfc3339(),
                warranty_status: status,
                days_remaining,
            }
        });

        items.push(MyProductItem {
            product_id: zeus_prod.id,
            product_name: zeus_prod.product_name,
            product_model_code: zeus_prod.product_model_code,
            serial_number: zeus_prod.serial_number,
            image_url,
            warranty,
        });
    }

    Ok(Json(ApiResponse::success(
        200,
        "Products retrieved successfully",
        items,
    )))
}
