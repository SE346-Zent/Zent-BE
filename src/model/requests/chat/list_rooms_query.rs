use serde::Deserialize;
use crate::model::requests::pagination::PaginationRequest;

#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ListRoomsQuery {
    #[serde(flatten)]
    pub pagination: PaginationRequest,
}
