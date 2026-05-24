use axum::{extract::{State, Path}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::product_detail_response::ProductDetailResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::get_product::{self, ProductWithRelations};
use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions, new_part_forms, part_audit_log, work_orders};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

/// Retrieve detailed information for a single product.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/products/{id}",
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
pub async fn get_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<ProductDetailResponse>>, AppError> {
    let zeus_prod = state.zeus_client.get_product(id).await?;
    let all_parts_list = state.zeus_client.list_parts(&ListPartsQuery {
        page: None,
        limit: Some(10000),
        ..Default::default()
    }).await?;

    let model_definition = product_models::Entity::find_by_id(zeus_prod.product_model_code.clone())
        .one(state.db.as_ref())
        .await?
        .unwrap_or_else(|| product_models::Model {
            model_code: zeus_prod.product_model_code.clone(),
            model_name: format!("Model {}", zeus_prod.product_model_code),
            description: None,
            created_at: zeus_prod.created_at,
            updated_at: zeus_prod.updated_at,
            deleted_at: None,
        });

    let prod_parts: Vec<_> = all_parts_list.items.iter()
        .filter(|p| p.product_id == Some(zeus_prod.id))
        .collect();

    let mut installed_parts = Vec::new();
    for p in prod_parts {
        let catalog_definition = part_catalog::Entity::find_by_id(p.part_catalog_id)
            .one(state.db.as_ref())
            .await?
            .unwrap_or_else(|| part_catalog::Model {
                id: p.part_catalog_id,
                part_number: "UNKNOWN".to_string(),
                part_types_id: 1,
                mfg_number: "UNKNOWN".to_string(),
                description: None,
                part_mfg_status: 1,
                created_at: p.created_at,
                updated_at: p.updated_at,
                deleted_at: None,
            });

        let physical_condition = part_conditions::Entity::find_by_id(p.part_condition_id)
            .one(state.db.as_ref())
            .await?
            .unwrap_or_else(|| part_conditions::Model {
                id: p.part_condition_id,
                name: "UNKNOWN".to_string(),
            });

        let form = new_part_forms::Entity::find()
            .filter(new_part_forms::Column::SerialNumber.eq(&p.serial_number))
            .one(state.db.as_ref())
            .await?;

        let mut approval_status = "approved".to_string();
        let mut registering_technician_id = None;
        if let Some(f) = form {
            let audit_log = part_audit_log::Entity::find()
                .filter(part_audit_log::Column::NewPartFormId.eq(f.id))
                .one(state.db.as_ref())
                .await?;
            if let Some(log) = audit_log {
                approval_status = log.action.clone();
            } else {
                approval_status = "pending".to_string();
            }
            if let Ok(Some(wo)) = work_orders::Entity::find_by_id(f.work_order_id).one(state.db.as_ref()).await {
                registering_technician_id = wo.technician_id;
            }
        }

        installed_parts.push(crate::services::v1::inventory::get_product::PartInProduct {
            part_record: parts::Model {
                id: p.id,
                part_catalog_id: p.part_catalog_id,
                product_id: p.product_id,
                serial_number: p.serial_number.clone(),
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
            product_model_code: zeus_prod.product_model_code.clone(),
            customer_id: zeus_prod.customer_id,
            product_name: zeus_prod.product_name.clone(),
            serial_number: zeus_prod.serial_number.clone(),
            created_at: zeus_prod.created_at,
            updated_at: zeus_prod.updated_at,
            deleted_at: None,
        },
        model_definition,
        installed_parts,
    };

    let detail = get_product::get_product_detail(&product_relation_data, &auth.role.name, auth.user.id)?;

    Ok(Json(ApiResponse::success(
        200,
        "Product retrieved successfully",
        detail,
    )))
}
