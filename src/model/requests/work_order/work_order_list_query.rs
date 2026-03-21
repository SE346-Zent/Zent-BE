use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;
use crate::model::responses::common::pagination::PaginationQuery;

#[derive(Deserialize, Debug, IntoParams)]
pub struct WorkOrderListQuery {
    pub user_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
