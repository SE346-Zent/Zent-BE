use axum::{extract::{State, Query}, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::responses::inventory::product_list_item::ProductListItem;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::list_products::{self, ProductEntry, PartInProduct};
use crate::entities::{products as prod, product_models, parts, part_catalog, part_conditions, new_part_forms, work_orders};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

pub async fn assemble_product_entries(
    db: &sea_orm::DatabaseConnection,
    zeus_client: &dyn crate::services::v1::inventory::ports::ZeusInventoryClient,
    zeus_products: Vec<crate::services::v1::inventory::ports::ZeusProduct>,
) -> Result<Vec<ProductEntry>, AppError> {
    // Fetch all parts once to avoid N+1 requests
    let all_parts_list = zeus_client.list_parts(&ListPartsQuery {
        page: None,
        limit: Some(10000),
        ..Default::default()
    }).await?;

    let mut entries = Vec::new();
    for zeus_prod in zeus_products {
        let model_definition = product_models::Entity::find_by_id(zeus_prod.product_model_code.clone())
            .one(db)
            .await?
            .unwrap_or_else(|| product_models::Model {
                model_code: zeus_prod.product_model_code.clone(),
                model_name: format!("Model {}", zeus_prod.product_model_code),
                description: None,
                created_at: zeus_prod.created_at,
                updated_at: zeus_prod.updated_at,
                deleted_at: None,
            });

        // Find parts installed in this product
        let prod_parts: Vec<_> = all_parts_list.items.iter()
            .filter(|p| p.product_id == Some(zeus_prod.id))
            .collect();

        let mut installed_parts = Vec::new();
        for p in prod_parts {
            let catalog_definition = part_catalog::Entity::find_by_id(p.part_catalog_id)
                .one(db)
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
                .one(db)
                .await?
                .unwrap_or_else(|| part_conditions::Model {
                    id: p.part_condition_id,
                    name: "UNKNOWN".to_string(),
                });

            // Registering technician
            let form = new_part_forms::Entity::find()
                .filter(new_part_forms::Column::SerialNumber.eq(&p.serial_number))
                .one(db)
                .await?;

            let mut registering_technician_id = None;
            if let Some(f) = form {
                if let Ok(Some(wo)) = work_orders::Entity::find_by_id(f.work_order_id).one(db).await {
                    registering_technician_id = wo.technician_id;
                }
            }

            installed_parts.push(PartInProduct {
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
                registering_technician_id,
            });
        }

        entries.push(ProductEntry {
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
        });
    }
    Ok(entries)
}

/// Retrieve a filtered, paginated list of products.
#[utoipa::path(
    get,
    path = "/api/v1/inventory/products",
    params(ListProductsQuery),
    responses(
        (status = 200, description = "List of products retrieved successfully", body = ApiResponse<Vec<ProductListItem>>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_products(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListProductsQuery>,
) -> Result<Json<ApiResponse<Vec<ProductListItem>>>, AppError> {
    let zeus_query = ListProductsQuery {
        model_code: query.model_code.clone(),
        search: query.search.clone(),
        page: None,
        limit: Some(10000), // Fetch a large batch to filter/paginate in memory
        sort_by: None,
        sort_order: None,
    };

    let zeus_list = state.zeus_client.list_products(&zeus_query).await?;
    let assembled = assemble_product_entries(state.db.as_ref(), &*state.zeus_client, zeus_list.items).await?;

    let (items, meta) = list_products::list_products(&assembled, &auth.role.name, auth.user.id, &query);

    Ok(Json(ApiResponse::success_with_meta(
        200,
        "Products retrieved successfully",
        items,
        meta,
    )))
}
