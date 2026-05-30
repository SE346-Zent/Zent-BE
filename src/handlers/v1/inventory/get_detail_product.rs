use axum::{extract::{State, Path}, Json};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::core::errors::{AppError, ErrorResponse};
use crate::core::state::AppState;
use crate::entities::{
    new_part_forms, part_catalog, part_conditions, parts,
    product_models, products as prod, warranties, work_orders,
};
use crate::extractor::auth_user::AuthUser;
use crate::model::responses::base::ApiResponse;
use crate::model::responses::inventory::product_detail_response::{
    ProductDetailResponse, ProductWarrantySummary, ProductWorkOrderHistoryItem,
};
use crate::services::v1::inventory::get_product::{self, ProductWithRelations};

/// Retrieve detailed information for a single product.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/products/{id}",
    tag = "inventory",
    params(
        ("id" = Uuid, Path, description = "The unique identifier of the product")
    ),
    responses(
        (status = 200, description = "Detailed product information retrieved successfully", body = ApiResponse<ProductDetailResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Product not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_detail_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<ProductDetailResponse>>, AppError> {
    let zeus_prod = state.zeus_client.get_product(id).await?;
    let product_parts = state.zeus_client.find_parts_by_product(zeus_prod.id).await?;

    let (model_definition, product_image_url) = match state
        .zeus_client
        .get_product_model(&zeus_prod.product_model_code)
        .await
    {
        Ok(model) => (
            product_models::Model {
                model_code: model.model_code,
                model_name: model.model_name,
                description: model.description,
                created_at: zeus_prod.created_at,
                updated_at: zeus_prod.updated_at,
                deleted_at: None,
            },
            model.image_url,
        ),
        Err(_) => (
            product_models::Model {
                model_code: zeus_prod.product_model_code.clone(),
                model_name: format!("Model {}", zeus_prod.product_model_code),
                description: None,
                created_at: zeus_prod.created_at,
                updated_at: zeus_prod.updated_at,
                deleted_at: None,
            },
            None,
        ),
    };

    let warranty = warranties::Entity::find()
        .filter(warranties::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?
        .map(|item| {
            let now = Utc::now();
            let status_name = item
                .warranty_status_id
                .and_then(|status_id| state.lookup_tables.warranty_statuses.get(&status_id).cloned())
                .unwrap_or_else(|| item.warranty_status.clone());
            let is_voided = status_name.eq_ignore_ascii_case("voided");
            let is_expired = now > item.end_date;
            let warranty_status = if is_voided {
                "Voided".to_string()
            } else if is_expired {
                "Expired".to_string()
            } else {
                "In Warranty".to_string()
            };
            let support_days_remaining = if is_voided || is_expired {
                0
            } else {
                (item.end_date - now).num_days().max(0)
            };

            ProductWarrantySummary {
                support_status: if support_days_remaining > 0 {
                    format!("{} days remaining", support_days_remaining)
                } else {
                    warranty_status.clone()
                },
                warranty_status,
                support_days_remaining,
                start_date: Some(item.start_date.to_rfc3339()),
                end_date: Some(item.end_date.to_rfc3339()),
            }
        });

    let work_order_history = work_orders::Entity::find()
        .filter(work_orders::Column::DeletedAt.is_null())
        .filter(work_orders::Column::ProductId.eq(zeus_prod.id))
        .order_by_desc(work_orders::Column::CreatedAt)
        .limit(10)
        .all(state.db.as_ref())
        .await?
        .into_iter()
        .map(|wo| {
            let status = state
                .lookup_tables
                .work_order_statuses
                .get(&wo.work_order_status_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string());

            ProductWorkOrderHistoryItem {
                work_order_id: wo.id,
                work_order_number: wo.work_order_number,
                status,
                date: crate::utils::time::to_utc7_string(wo.created_at),
            }
        })
        .collect::<Vec<_>>();

    let mut installed_parts = Vec::new();
    for p in product_parts {
        let catalog_definition = match state.zeus_client.get_part_catalog(p.part_catalog_id).await {
            Ok(c) => part_catalog::Model {
                id: c.id,
                part_number: c.part_number,
                part_types_id: c.part_types_id,
                mfg_number: c.mfg_number,
                description: c.description,
                part_mfg_status: c.part_mfg_status,
                created_at: p.created_at,
                updated_at: p.updated_at,
                deleted_at: None,
            },
            Err(_) => part_catalog::Model {
                id: p.part_catalog_id,
                part_number: "UNKNOWN".to_string(),
                part_types_id: 0,
                mfg_number: "UNKNOWN".to_string(),
                description: None,
                part_mfg_status: 0,
                created_at: p.created_at,
                updated_at: p.updated_at,
                deleted_at: None,
            },
        };

        let physical_condition = part_conditions::Model {
            id: p.part_condition_id,
            name: format!("Condition {}", p.part_condition_id),
        };

        let form = new_part_forms::Entity::find()
            .filter(new_part_forms::Column::SerialNumber.eq(&p.serial_number))
            .one(state.db.as_ref())
            .await?;

        let mut approval_status = "approved".to_string();
        let mut registering_technician_id = None;
        if let Some(f) = form {
            approval_status = if f.status.eq_ignore_ascii_case("denied") {
                "rejected".to_string()
            } else {
                f.status.clone()
            };
            if let Ok(Some(wo)) = work_orders::Entity::find_by_id(f.work_order_id).one(state.db.as_ref()).await {
                registering_technician_id = wo.technician_id;
            }
        }

        installed_parts.push(get_product::PartInProduct {
            part_record: parts::Model {
                id: p.id,
                part_catalog_id: p.part_catalog_id,
                product_id: p.product_id,
                serial_number: p.serial_number,
                part_condition_id: p.part_condition_id,
                manufactured_date: p.manufactured_date,
                installation_date: p.installation_date,
                removal_date: p.removal_date,
                scrapped_date: p.scrapped_date,
                created_at: p.created_at,
                updated_at: p.updated_at,
                deleted_at: None,
            },
            catalog_definition,
            physical_condition,
            approval_status,
            registering_technician_id,
        });
    }

    let product_relation_data = ProductWithRelations {
        product_record: prod::Model {
            id: zeus_prod.id,
            product_model_code: zeus_prod.product_model_code,
            customer_id: zeus_prod.customer_id,
            product_name: zeus_prod.product_name,
            serial_number: zeus_prod.serial_number,
            created_at: zeus_prod.created_at,
            updated_at: zeus_prod.updated_at,
            deleted_at: None,
        },
        model_definition,
        product_image_url,
        installed_parts,
        warranty,
        work_order_history,
    };

    let detail = get_product::get_product_detail(&product_relation_data, &auth.role.name, auth.user.id)?;

    Ok(Json(ApiResponse::success(
        200,
        "Product retrieved successfully",
        detail,
    )))
}