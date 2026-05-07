use geo::prelude::*;
use geo::Point;

pub fn is_within_geofence(
    tech_lat: f64,
    tech_lng: f64,
    target_lat: f64,
    target_lng: f64,
    radius_meters: f64,
) -> bool {
    let tech_point = Point::new(tech_lng, tech_lat);
    let target_point = Point::new(target_lng, target_lat);

    // Haversine distance in meters (explicitly using meters as requested)
    #[allow(deprecated)]
    let distance = tech_point.haversine_distance(&target_point);

    distance <= radius_meters
}
