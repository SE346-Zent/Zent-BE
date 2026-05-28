use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeofenceCheckRequest {
    /// The current latitude of the technician. Must be in the range [-90, 90].
    #[validate(range(min = -90.0, max = 90.0))]
    pub latitude: f64,
    /// The current longitude of the technician. Must be in the range [-180, 180].
    #[validate(range(min = -180.0, max = 180.0))]
    pub longitude: f64,
}
