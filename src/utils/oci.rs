use crate::core::config::AppConfig;
use crate::core::errors::AppError;
use reqwest::header::CONTENT_TYPE;

/// Upload a binary object to Oracle Cloud Infrastructure (OCI) Object Storage using a Pre-Authenticated Request (PAR).
///
/// Returns the name of the uploaded object on success.
pub async fn upload_object(
    object_name: &str,
    data: Vec<u8>,
    content_type: &str,
) -> Result<String, AppError> {
    let cfg = AppConfig::get();
    
    // OCI Bucket-level PARs usually allow appending the object name to the end of the URL
    // e.g., https://objectstorage.region.com/p/token/n/namespace/b/bucket/o/ + object_name
    let write_url = format!("{}{}", cfg.par_write_work_orders, object_name);

    let client = reqwest::Client::new();
    let resp = client.put(write_url)
        .header(CONTENT_TYPE, content_type)
        .body(data)
        .send()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to upload via PAR: {}", e)))?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(anyhow::anyhow!("OCI PAR upload error: {}", err_text)));
    }

    Ok(object_name.to_string())
}
