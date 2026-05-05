use serde::Deserialize;
use uuid::Uuid;
use crate::model::requests::pagination::PaginationRequest;

#[derive(Deserialize, Debug, utoipa::IntoParams, utoipa::ToSchema)]
pub struct WorkOrderQuery {
    #[serde(flatten)]
    pub pagination: PaginationRequest,
    
    /// Optional role context. If provided, it must match the user's role in the token.
    pub role: Option<String>,
    
    /// Optional filter by province (Super Admin only, or restricted to Admin's own province)
    pub province: Option<String>,
    
    /// Optional filter by technician ID (Super Admin and Admin only)
    pub technician_id: Option<Uuid>,
}
