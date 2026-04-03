use sea_orm::*;
use uuid::Uuid;

use crate::model::responses::common::pagination_meta::PaginationMeta;

use crate:: {
    entities::products,
    model:: {
        requests::product::product_list_query::ProductListQuery,
        responses::error::AppError,
        responses::product::product_list_item_response::{ProductListResponse, ProductListItemResponseData}
    }
};

pub async fn get_product_list_service(
    db: DatabaseConnection,
    query: ProductListQuery
) -> Result<ProductListResponse, AppError> {
    let db_query = products::Entity::find()
        .filter(products::Column::CustomerId.eq(query.customer_id));

    let total_items = db_query
        .clone()
        .count(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let paginator = db_query.paginate(&db, query.pagination.per_page);
    let page_index = query.pagination.page.saturating_sub(1);

    let products_list = paginator
        .fetch_page(page_index)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = products_list.into_iter().map(map_product_list_item).collect::<Vec<_>>();
    let meta = PaginationMeta::new(query.pagination.page, query.pagination.per_page, total_items);
    Ok(ProductListResponse::success(data, meta))
}

fn map_product_list_item(model: products::Model) -> ProductListItemResponseData {
    ProductListItemResponseData {
        id: model.id,
        model_id: model.model_id,
        customer_id: model.customer_id,
        product_name: model.product_name,
        serial_number: model.serial_number,
        created_at: model.created_at.to_string(),
        updated_at: model.updated_at.to_string(),
        deleted_at: model.deleted_at.map(|d| d.to_string()),
    }
}
