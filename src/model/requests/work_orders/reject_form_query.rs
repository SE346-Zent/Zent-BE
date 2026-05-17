use serde::Deserialize;
use crate::model::requests::pagination::PaginationRequest;

#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct RejectFormQuery {
    #[serde(flatten)]
    pub pagination: PaginationRequest,
}
