pub mod models;
pub mod parts;
pub mod products;

use crate::core::errors::AppError;
use crate::services::v1::inventory::ports::{
    ZeusInventoryClient, ZeusPart, ZeusPartCatalog, ZeusProduct, ZeusProductModel,
    ZeusLutCollection,
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

    fn make_put(&self, path: &str) -> reqwest::RequestBuilder {
        self.client
            .put(format!("{}{}", self.base_url, path))
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

    async fn install_part(&self, part_id: Uuid, product_id: Uuid) -> Result<(), AppError> {
        let payload = PartsApi::install_part_payload(product_id);
        let _: ZeusEnvelope<serde_json::Value> = self
            .send_expect_envelope(
                self.make_post(&format!("/inventory/parts/{}/install", part_id)).json(&payload),
            )
            .await?;
        Ok(())
    }

    async fn remove_part(&self, part_id: Uuid) -> Result<(), AppError> {
        let _: ZeusEnvelope<serde_json::Value> = self
            .send_expect_envelope(
                self.make_post(&format!("/inventory/parts/{}/remove", part_id)),
            )
            .await?;
        Ok(())
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

    async fn update_product(
        &self,
        id: Uuid,
        model_code: &str,
        customer_id: Uuid,
        product_name: &str,
        serial_number: &str,
    ) -> Result<ZeusProduct, AppError> {
        let payload = ProductsApi::create_product_payload(model_code, customer_id, product_name, serial_number);
        let envelope: ZeusEnvelope<models::ZeusProductDto> = self
            .send_expect_envelope(
                self.make_put(&format!("/inventory/products/{}", id)).json(&payload),
            )
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Failed to retrieve updated product data from Zeus"
            ))
        })?;
        Ok(ProductsApi::to_domain(data))
    }

    async fn list_products(&self) -> Result<Vec<ZeusProduct>, AppError> {
        let envelope: ZeusEnvelope<Vec<models::ZeusProductDto>> = self
            .send_expect_envelope(self.make_get("/inventory/products"))
            .await?;

        let data = envelope.data.unwrap_or_default();
        Ok(data.into_iter().map(ProductsApi::to_domain).collect())
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

    async fn get_part_catalog(&self, id: Uuid) -> Result<ZeusPartCatalog, AppError> {
        let envelope = self
            .send_expect_envelope_or_not_found(
                self.make_get(&format!("/inventory/part-catalog/{}", id)),
                "Part catalog",
                id,
            )
            .await?;

        let data: models::ZeusPartCatalogDto = envelope.data.ok_or_else(|| {
            AppError::NotFound(format!("Part catalog with ID {} not found in Zeus", id))
        })?;

        Ok(ZeusPartCatalog {
            id: data.id,
            part_number: data.part_number,
            part_types_id: data.part_types_id,
            mfg_number: data.mfg_number,
            description: data.description,
            part_mfg_status: data.part_mfg_status,
        })
    }

    async fn update_part_catalog_by_sku(
        &self,
        sku: &str,
        description: Option<&str>,
        part_mfg_status: Option<i32>,
    ) -> Result<ZeusPartCatalog, AppError> {
        let mut payload = serde_json::Map::new();
        if let Some(description) = description {
            payload.insert("description".to_string(), serde_json::Value::String(description.to_string()));
        }
        if let Some(status) = part_mfg_status {
            payload.insert("part_mfg_status".to_string(), serde_json::Value::Number(status.into()));
        }

        let envelope: ZeusEnvelope<models::ZeusPartCatalogDto> = self
            .send_expect_envelope(
                self.make_put(&format!("/inventory/part-catalog/{}", sku)).json(&payload),
            )
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::NotFound(format!("Part catalog with SKU {} not found in Zeus", sku))
        })?;

        Ok(ZeusPartCatalog {
            id: data.id,
            part_number: data.part_number,
            part_types_id: data.part_types_id,
            mfg_number: data.mfg_number,
            description: data.description,
            part_mfg_status: data.part_mfg_status,
        })
    }

    async fn create_part_catalog(&self, part_number: &str, part_types_id: i32, mfg_number: &str, description: Option<&str>, part_mfg_status: i32) -> Result<ZeusPartCatalog, AppError> {
        let mut payload = serde_json::Map::new();
        payload.insert("part_number".to_string(), serde_json::Value::String(part_number.to_string()));
        payload.insert("part_types_id".to_string(), serde_json::Value::Number(part_types_id.into()));
        payload.insert("mfg_number".to_string(), serde_json::Value::String(mfg_number.to_string()));
        payload.insert("part_mfg_status".to_string(), serde_json::Value::Number(part_mfg_status.into()));
        if let Some(desc) = description {
            payload.insert("description".to_string(), serde_json::Value::String(desc.to_string()));
        }

        let envelope: ZeusEnvelope<models::ZeusPartCatalogDto> = self
            .send_expect_envelope(self.make_post("/inventory/part-catalog").json(&payload))
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed to create part catalog in Zeus"))
        })?;

        Ok(ZeusPartCatalog {
            id: data.id,
            part_number: data.part_number,
            part_types_id: data.part_types_id,
            mfg_number: data.mfg_number,
            description: data.description,
            part_mfg_status: data.part_mfg_status,
        })
    }

    async fn find_part_catalog_by_part_number(&self, part_number: &str) -> Result<Option<ZeusPartCatalog>, AppError> {
        let envelope: ZeusEnvelope<serde_json::Value> = self
            .send_expect_envelope(
                self.make_get("/inventory/part-catalog")
                    .query(&[("q", part_number), ("limit", "10")]),
            )
            .await?;

        let items: Vec<models::ZeusPartCatalogDto> = envelope
            .data
            .and_then(|d| d.get("items").cloned())
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        let found = items.into_iter().find(|i| i.part_number == part_number);

        Ok(found.map(|data| ZeusPartCatalog {
            id: data.id,
            part_number: data.part_number,
            part_types_id: data.part_types_id,
            mfg_number: data.mfg_number,
            description: data.description,
            part_mfg_status: data.part_mfg_status,
        }))
    }

    async fn get_product_model(&self, code: &str) -> Result<ZeusProductModel, AppError> {
        let envelope: ZeusEnvelope<models::ZeusProductModelDto> = self
            .send_expect_envelope(
                self.make_get(&format!("/inventory/product-models/{}", code)),
            )
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::NotFound(format!("Product model with code {} not found in Zeus", code))
        })?;

        Ok(ZeusProductModel {
            model_code: data.model_code,
            model_name: data.model_name,
            description: data.description,
            image_url: data.image_url,
        })
    }

    async fn get_luts(&self) -> Result<ZeusLutCollection, AppError> {
        let envelope: ZeusEnvelope<ZeusLutCollection> = self
            .send_expect_envelope(self.make_get("/luts"))
            .await?;

        let data = envelope.data.ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Failed to retrieve LUTs from SCM"))
        })?;

        Ok(data)
    }
}
