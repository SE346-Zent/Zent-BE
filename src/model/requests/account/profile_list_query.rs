use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use crate::model::responses::common::pagination_meta::PaginationQuery;

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub enum RoleQuery {
    Technician,
    Admin,
}

impl ToString for RoleQuery {
    fn to_string(&self) -> String {
        match self {
            RoleQuery::Technician => "Technician".to_string(),
            RoleQuery::Admin => "Admin".to_string(),
        }
    }
}

/// Query parameters for the profile list endpoint.
/// Example: `GET /profiles?role=Technician&page=2&per_page=20`
#[derive(Deserialize, Debug, IntoParams)]
pub struct ProfileListQuery {
    pub role: RoleQuery,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}
