use serde::Deserialize;
use crate::core::config::AppConfig;
use crate::core::errors::AppError;

#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Vec<GeocodingResult>,
    status: String,
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeocodingResult {
    geometry: Geometry,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    location: Location,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Location {
    pub lat: f64,
    pub lng: f64,
}

pub async fn geocode_address(
    address: &str,
    city: &str,
    province: &str,
    country: &str,
) -> Result<Location, AppError> {
    let cfg = AppConfig::get();
    let full_address = format!("{}, {}, {}, {}", address, city, province, country);
    let url = format!(
        "https://maps.googleapis.com/maps/api/geocode/json?address={}&key={}",
        urlencoding::encode(&full_address),
        cfg.google_maps_api_key
    );

    let client = reqwest::Client::new();
    let resp: GeocodingResponse = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to call Google Maps API: {}", e)))?
        .json()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Google Maps response: {}", e)))?;

    if resp.status != "OK" {
        if resp.status == "ZERO_RESULTS" {
            return Err(AppError::BadRequest("Address not found".to_string()));
        }
        let detailed_error = resp.error_message.unwrap_or_else(|| "No detailed message provided".to_string());
        return Err(AppError::Internal(anyhow::anyhow!("Google Maps API error: {} - {}", resp.status, detailed_error)));
    }

    resp.results
        .first()
        .map(|r| r.geometry.location)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No results from Google Maps API")))
}
