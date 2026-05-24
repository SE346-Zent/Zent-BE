use crate::core::errors::AppError;
use crate::services::v1::inventory::ports::{
    ZeusInventoryClient, ZeusPart, ZeusPartList, ZeusProduct, ZeusProductList,
};
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use uuid::Uuid;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Deserialize)]
struct ZeusEnvelope<T> {
    data: Option<T>,
    message: String,
    statusCode: u16,
    metadata: Option<ZeusMetadata>,
}

#[derive(Debug, Deserialize)]
struct ZeusMetadata {
    pagination: Option<ZeusPaginationDto>,
}

#[derive(Debug, Deserialize)]
struct ZeusPaginationDto {
    page: u64,
    limit: u64,
    total_rows: u64,
    total_pages: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ZeusPartDto {
    #[serde(rename = "ID")]
    id: Uuid,
    #[serde(rename = "PartCatalogID")]
    part_catalog_id: Uuid,
    #[serde(rename = "PartConditionID")]
    part_condition_id: i32,
    #[serde(rename = "ProductID")]
    product_id: Option<Uuid>,
    pub serial_number: String,
    pub manufactured_date: chrono::DateTime<chrono::Utc>,
    pub installation_date: Option<chrono::DateTime<chrono::Utc>>,
    pub removal_date: Option<chrono::DateTime<chrono::Utc>>,
    pub scrapped_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ZeusProductDto {
    #[serde(rename = "ID")]
    id: Uuid,
    pub product_model_code: String,
    #[serde(rename = "CustomerID")]
    pub customer_id: Uuid,
    pub product_name: String,
    pub serial_number: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
impl ZeusInventoryClient for ZeusClient {
    async fn list_parts(&self, query: &ListPartsQuery) -> Result<ZeusPartList, AppError> {
        let mut query_params: Vec<(&str, String)> = Vec::new();
        if let Some(ref search) = query.search {
            query_params.push(("q", search.clone()));
        }
        if let Some(page) = query.page {
            query_params.push(("page", page.to_string()));
        }
        if let Some(limit) = query.limit {
            query_params.push(("limit", limit.to_string()));
        }
        if let Some(ref sort_by) = query.sort_by {
            let sort_field = match sort_by.as_str() {
                "created_at" => "created_at",
                "serial_number" => "serial_number",
                "part_condition_id" => "part_condition_id",
                _ => "created_at",
            };
            query_params.push(("sort_by", sort_field.to_string()));
        }
        if let Some(ref sort_order) = query.sort_order {
            let sort_dir = match sort_order.as_str() {
                "asc" => "asc",
                "desc" => "desc",
                _ => "desc",
            };
            query_params.push(("sort_dir", sort_dir.to_string()));
        }

        let mut req = self.client.get(format!("{}/inventory/parts", self.base_url))
            .header("X-API-KEY", &self.api_key);

        if !query_params.is_empty() {
            req = req.query(&query_params);
        }

        let res = req.send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<Vec<ZeusPartDto>> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let data = body.data.unwrap_or_default();
        let items = data.into_iter().map(|dto| ZeusPart {
            id: dto.id,
            part_catalog_id: dto.part_catalog_id,
            part_condition_id: dto.part_condition_id,
            product_id: dto.product_id,
            serial_number: dto.serial_number,
            manufactured_date: dto.manufactured_date,
            installation_date: dto.installation_date,
            removal_date: dto.removal_date,
            scrapped_date: dto.scrapped_date,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }).collect();

        let (total_rows, total_pages) = if let Some(meta) = body.metadata.and_then(|m| m.pagination) {
            (meta.total_rows, meta.total_pages)
        } else {
            (0, 0)
        };

        Ok(ZeusPartList {
            items,
            total_rows,
            total_pages,
        })
    }

    async fn get_part(&self, id: Uuid) -> Result<ZeusPart, AppError> {
        let res = self.client.get(format!("{}/inventory/parts/{}", self.base_url, id))
            .header("X-API-KEY", &self.api_key)
            .send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!("Part with ID {} not found in Zeus", id)));
        }
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<ZeusPartDto> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let dto = body.data.ok_or_else(|| AppError::NotFound(format!("Part with ID {} not found in Zeus", id)))?;
        Ok(ZeusPart {
            id: dto.id,
            part_catalog_id: dto.part_catalog_id,
            part_condition_id: dto.part_condition_id,
            product_id: dto.product_id,
            serial_number: dto.serial_number,
            manufactured_date: dto.manufactured_date,
            installation_date: dto.installation_date,
            removal_date: dto.removal_date,
            scrapped_date: dto.scrapped_date,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
    }

    async fn create_part(&self, part_catalog_id: Uuid, condition_id: i32, serial_number: &str, mfg_date: chrono::DateTime<chrono::Utc>) -> Result<ZeusPart, AppError> {
        #[derive(Serialize)]
        struct CreatePartPayload {
            part_catalog_id: Uuid,
            part_condition_id: i32,
            serial_number: String,
            manufactured_date: chrono::DateTime<chrono::Utc>,
        }

        let payload = CreatePartPayload {
            part_catalog_id,
            part_condition_id: condition_id,
            serial_number: serial_number.to_string(),
            manufactured_date: mfg_date,
        };

        let res = self.client.post(format!("{}/inventory/parts", self.base_url))
            .header("X-API-KEY", &self.api_key)
            .json(&payload)
            .send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<ZeusPartDto> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let dto = body.data.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to retrieve created part data from Zeus")))?;
        Ok(ZeusPart {
            id: dto.id,
            part_catalog_id: dto.part_catalog_id,
            part_condition_id: dto.part_condition_id,
            product_id: dto.product_id,
            serial_number: dto.serial_number,
            manufactured_date: dto.manufactured_date,
            installation_date: dto.installation_date,
            removal_date: dto.removal_date,
            scrapped_date: dto.scrapped_date,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
    }

    async fn list_products(&self, query: &ListProductsQuery) -> Result<ZeusProductList, AppError> {
        let mut query_params: Vec<(&str, String)> = Vec::new();
        if let Some(ref search) = query.search {
            query_params.push(("q", search.clone()));
        }
        if let Some(page) = query.page {
            query_params.push(("page", page.to_string()));
        }
        if let Some(limit) = query.limit {
            query_params.push(("limit", limit.to_string()));
        }
        if let Some(ref sort_by) = query.sort_by {
            let sort_field = match sort_by.as_str() {
                "created_at" => "created_at",
                "product_name" => "product_name",
                "serial_number" => "serial_number",
                _ => "created_at",
            };
            query_params.push(("sort_by", sort_field.to_string()));
        }
        if let Some(ref sort_order) = query.sort_order {
            let sort_dir = match sort_order.as_str() {
                "asc" => "asc",
                "desc" => "desc",
                _ => "desc",
            };
            query_params.push(("sort_dir", sort_dir.to_string()));
        }

        let mut req = self.client.get(format!("{}/inventory/products", self.base_url))
            .header("X-API-KEY", &self.api_key);

        if !query_params.is_empty() {
            req = req.query(&query_params);
        }

        let res = req.send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<Vec<ZeusProductDto>> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let data = body.data.unwrap_or_default();
        let items = data.into_iter().map(|dto| ZeusProduct {
            id: dto.id,
            product_model_code: dto.product_model_code,
            customer_id: dto.customer_id,
            product_name: dto.product_name,
            serial_number: dto.serial_number,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }).collect();

        let (total_rows, total_pages) = if let Some(meta) = body.metadata.and_then(|m| m.pagination) {
            (meta.total_rows, meta.total_pages)
        } else {
            (0, 0)
        };

        Ok(ZeusProductList {
            items,
            total_rows,
            total_pages,
        })
    }

    async fn get_product(&self, id: Uuid) -> Result<ZeusProduct, AppError> {
        let res = self.client.get(format!("{}/inventory/products/{}", self.base_url, id))
            .header("X-API-KEY", &self.api_key)
            .send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(format!("Product with ID {} not found in Zeus", id)));
        }
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<ZeusProductDto> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let dto = body.data.ok_or_else(|| AppError::NotFound(format!("Product with ID {} not found in Zeus", id)))?;
        Ok(ZeusProduct {
            id: dto.id,
            product_model_code: dto.product_model_code,
            customer_id: dto.customer_id,
            product_name: dto.product_name,
            serial_number: dto.serial_number,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
    }

    async fn create_product(&self, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError> {
        #[derive(Serialize)]
        struct CreateProductPayload {
            product_model_code: String,
            customer_id: Uuid,
            product_name: String,
            serial_number: String,
        }

        let payload = CreateProductPayload {
            product_model_code: model_code.to_string(),
            customer_id,
            product_name: product_name.to_string(),
            serial_number: serial_number.to_string(),
        };

        let res = self.client.post(format!("{}/inventory/products", self.base_url))
            .header("X-API-KEY", &self.api_key)
            .json(&payload)
            .send().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send request to Zeus: {}", e)))?;

        let status = res.status();
        if !status.is_success() {
            let err_body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(anyhow::anyhow!("Zeus API error: {} - {}", status, err_body)));
        }

        let body: ZeusEnvelope<ZeusProductDto> = res.json().await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Zeus response: {}", e)))?;

        let dto = body.data.ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to retrieve created product data from Zeus")))?;
        Ok(ZeusProduct {
            id: dto.id,
            product_model_code: dto.product_model_code,
            customer_id: dto.customer_id,
            product_name: dto.product_name,
            serial_number: dto.serial_number,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        })
    }
}
