use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::model::responses::common::pagination_meta::PaginationQuery;

#[derive(Deserialize, Debug, IntoParams)]
pub struct PartListQuery {
    #[serde(rename = "productId")]
    pub product_id: Uuid,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
