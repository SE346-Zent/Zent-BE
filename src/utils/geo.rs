use geo::prelude::*;
use geo::Point;

/// Check if a technician's location is within a specific geofence radius of a target location.
///
/// # Arguments
/// * `tech_latitude` - The latitude of the technician's current location.
/// * `tech_longitude` - The longitude of the technician's current location.
/// * `target_latitude` - The latitude of the target destination (e.g., service address).
/// * `target_longitude` - The longitude of the target destination.
/// * `radius_meters` - The maximum allowed distance in meters.
///
/// # Returns
/// `true` if the distance is less than or equal to the radius, `false` otherwise.
pub fn is_within_geofence(
    tech_latitude: f64,
    tech_longitude: f64,
    target_latitude: f64,
    target_longitude: f64,
    radius_meters: f64,
) -> bool {
    let tech_point = Point::new(tech_longitude, tech_latitude);
    let target_point = Point::new(target_longitude, target_latitude);

    // Haversine distance in meters (explicitly using meters as requested)
    #[allow(deprecated)]
    let distance = tech_point.haversine_distance(&target_point);

    distance <= radius_meters
}
