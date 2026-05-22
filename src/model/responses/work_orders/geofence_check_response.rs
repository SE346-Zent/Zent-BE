use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeofenceCheckResponse {
    /// True if the technician's coordinates are within the work site's geofence.
    pub is_within: bool,
    /// The actual calculated distance in meters between the technician and the work site.
    pub distance_meters: f64,
    /// The maximum allowed radius in meters for this geofence.
    pub allowed_radius_meters: f64,
}
