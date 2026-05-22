use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeofenceCheckRequest {
    /// The current latitude of the technician.
    pub latitude: f64,
    /// The current longitude of the technician.
    pub longitude: f64,
}
