pub mod models;
pub mod parts;
pub mod products;

use crate::core::errors::AppError;
use crate::services::v1::inventory::ports::{
    ZeusInventoryClient, ZeusPart, ZeusProduct,
};
use uuid::Uuid;
use reqwest::Client;

use self::models::ZeusEnvelope;
use self::parts::PartsApi;
use self::products::ProductsApi;

#[derive(Debug, Clone)]
pub struct ZeusClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ZeusClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    fn make_get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{}", self.base_url, path))
            .header("X-API-KEY", &self.api_key)
    }

    fn make_post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}{}", self.base_url, path))
            .header("X-API-KEY", &self.api_key)
    }

    async fn send_expect_envelope<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<ZeusEnvelope<T>, AppError> {
        let res = req.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e))
        })?;

        let status = res.status();
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!(
                "Zeus API error: {} - {}",
                status,
                err_body
            )));
        }

        res.json().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e))
        })
    }

    async fn send_expect_envelope_or_not_found<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        entity_label: &str,
        id: Uuid,
    ) -> Result<ZeusEnvelope<T>, AppError> {
        let res = req.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e))
        })?;

        let status = res.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!(
                "{} with ID {} not found in Zeus",
                entity_label, id
            )));
        }
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!(
                "Zeus API error: {} - {}",
                status,
                err_body
            )));
        }

        res.json().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e))
        })
    }
}

#[async_trait::async_trait]
impl ZeusInventoryClient for ZeusClient {
    async fn get_part(&self, id: Uuid) -> Result<ZeusPart, AppError> {
        let envelope = self
            .send_expect_envelope_or_not_found(
                self.make_get(&format!("/inventory/parts/{}", id)),
                "Part",
                id,
            )
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::NotFound(format!("Part with ID {} not found in Zeus", id))
        })?;
        Ok(PartsApi::to_domain(data))
    }

    async fn create_part(
        &self,
        part_catalog_id: Uuid,
        condition_id: i32,
        serial_number: &str,
        mfg_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<ZeusPart, AppError> {
        let payload = PartsApi::create_part_payload(part_catalog_id, condition_id, serial_number, mfg_date);
        let envelope = self
            .send_expect_envelope(self.make_post("/inventory/parts").json(&payload))
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to retrieve created part data from Zeus"
            ))
        })?;
        Ok(PartsApi::to_domain(data))
    }

    async fn find_product_by_serial(
        &self,
        serial_number: &str,
    ) -> Result<Option<ZeusProduct>, AppError> {
        let envelope: ZeusEnvelope<Vec<models::ZeusProductDto>> = self
            .send_expect_envelope(
                self.make_get("/inventory/products")
                    .query(&[("q", serial_number)]),
            )
            .await?;

        let data = envelope.data.unwrap_or_default();
        Ok(data
            .into_iter()
            .find(|p| p.serial_number == serial_number)
            .map(ProductsApi::to_domain))
    }

    async fn get_product(&self, id: Uuid) -> Result<ZeusProduct, AppError> {
        let envelope = self
            .send_expect_envelope_or_not_found(
                self.make_get(&format!("/inventory/products/{}", id)),
                "Product",
                id,
            )
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::NotFound(format!("Product with ID {} not found in Zeus", id))
        })?;
        Ok(ProductsApi::to_domain(data))
    }

    async fn create_product(
        &self,
        model_code: &str,
        customer_id: Uuid,
        product_name: &str,
        serial_number: &str,
    ) -> Result<ZeusProduct, AppError> {
        let payload = ProductsApi::create_product_payload(model_code, customer_id, product_name, serial_number);
        let envelope = self
            .send_expect_envelope(self.make_post("/inventory/products").json(&payload))
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to retrieve created product data from Zeus"
            ))
        })?;
        Ok(ProductsApi::to_domain(data))
    }

    async fn find_parts_by_product(
        &self,
        product_id: Uuid,
    ) -> Result<Vec<ZeusPart>, AppError> {
        let envelope: ZeusEnvelope<Vec<models::ZeusPartDto>> = self
            .send_expect_envelope(
                self.make_get("/inventory/parts")
                    .query(&[("product_id", product_id.to_string())]),
            )
            .await?;

        let data = envelope.data.unwrap_or_default();
        Ok(data.into_iter().map(PartsApi::to_domain).collect())
    }
}
