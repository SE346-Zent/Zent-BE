use serde::Deserialize;
use crate::core::config::AppConfig;
use crate::core::errors::AppError;

/// Timeout for the geocoding HTTP request (connect + response).
const GEOCODING_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, serde::Deserialize, Clone, Copy)]
pub struct Location {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
}

/// Classify a reqwest error into a structured AppError.
fn classify_reqwest_error(e: reqwest::Error, label: &str) -> AppError {
    if e.is_timeout() || e.is_connect() {
        AppError::ServiceUnavailable(format!("{}: upstream timed out or unreachable", label))
    } else if let Some(status) = e.status() {
        if status.is_server_error() {
            AppError::ServiceUnavailable(format!("{}: upstream returned {}", label, status))
        } else if status.is_client_error() {
            AppError::ServiceUnavailable(format!("{}: upstream rejected request ({})", label, status))
        } else {
            AppError::Internal(anyhow::anyhow!("{}: unexpected HTTP status {}", label, status))
        }
    } else {
        // Non-HTTP error (builder, body, JSON parse, etc.)
        AppError::ServiceUnavailable(format!("{}: {}", label, e))
    }
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
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencoding::encode(&full_address)
    );

    let client = reqwest::Client::builder()
        .user_agent(&cfg.nominatim_user_agent)
        .timeout(std::time::Duration::from_secs(GEOCODING_TIMEOUT_SECS))
        .build()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to build reqwest client: {}", e)))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| classify_reqwest_error(e, "Geocoding request"))?;

    // Check HTTP status before attempting to parse JSON
    let status = response.status();
    if !status.is_success() {
        return if status.is_server_error() {
            Err(AppError::ServiceUnavailable(format!(
                "Geocoding service unavailable (HTTP {})", status
            )))
        } else {
            Err(AppError::ServiceUnavailable(format!(
                "Geocoding request failed (HTTP {})", status
            )))
        };
    }

    let resp: Vec<NominatimResponse> = response
        .json()
        .await
        .map_err(|e| classify_reqwest_error(e, "Geocoding response parse"))?;

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
