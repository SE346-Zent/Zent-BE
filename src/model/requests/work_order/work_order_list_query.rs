use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;
use crate::model::responses::common::pagination::PaginationQuery;

#[derive(Deserialize, Debug, IntoParams)]
pub struct WorkOrderListQuery {
    pub admin_id: Uuid,
    pub customer_id: Uuid,
    pub technician_id: Uuid,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
