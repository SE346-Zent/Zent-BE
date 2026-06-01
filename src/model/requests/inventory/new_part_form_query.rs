use serde::Deserialize;
use utoipa::IntoParams;

use crate::model::requests::pagination::PaginationRequest;

#[derive(Debug, Deserialize, IntoParams)]
pub struct NewPartFormQuery {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationRequest,
}