use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::model::responses::common::pagination_meta::PaginationQuery;

#[derive(Deserialize, IntoParams, Debug)]
pub struct ProductListQuery {
    #[serde(rename = "customerId")]
    pub customer_id: Uuid,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
