use serde::Deserialize;
use crate::core::config::AppConfig;
use crate::core::errors::AppError;

/// Represents a geographical location with latitude and longitude coordinates.
#[derive(Debug, serde::Deserialize, Clone, Copy)]
pub struct Location {
    /// Latitude coordinate.
    pub lat: f64,
    /// Longitude coordinate.
    pub lng: f64,
}

#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
}

/// Convert a structured physical address into geographical coordinates (lat/lng) using the Nominatim API.
pub async fn geocode_address(
    address: &str,
    city: &str,
    province: &str,
    country: &str,
) -> Result<Location, AppError> {
    let cfg = AppConfig::get();
    let full_address = format!("{}, {}, {}, {}", address, city, province, country);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencoding::encode(&full_address)
    );

    let client = reqwest::Client::builder()
        .user_agent(&cfg.nominatim_user_agent)
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to build reqwest client: {}", e)))?;

    let resp: Vec<NominatimResponse> = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to call Nominatim API: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Nominatim response: {}", e)))?;

    let first = resp.first().ok_or_else(|| {
        AppError::BadRequest(format!("Address not found: '{}'", full_address))
    })?;

    let lat: f64 = first.lat.parse().unwrap_or(0.0);
    let lon: f64 = first.lon.parse().unwrap_or(0.0);

    tracing::info!(
        "Geocoded address: '{}' -> Coordinates: (lat: {}, lon: {})",
        full_address,
        lat,
        lon
    );

    Ok(Location {
        lat,
        lng: lon,
    })
}
