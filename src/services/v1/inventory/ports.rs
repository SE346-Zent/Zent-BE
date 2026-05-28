use crate::core::errors::AppError;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusPart {
    pub id: Uuid,
    pub part_catalog_id: Uuid,
    pub part_condition_id: i32,
    pub product_id: Option<Uuid>,
    pub serial_number: String,
    pub manufactured_date: chrono::DateTime<chrono::Utc>,
    pub installation_date: Option<chrono::DateTime<chrono::Utc>>,
    pub removal_date: Option<chrono::DateTime<chrono::Utc>>,
    pub scrapped_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusProduct {
    pub id: Uuid,
    pub product_model_code: String,
    pub customer_id: Uuid,
    pub product_name: String,
    pub serial_number: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusPartCatalog {
    pub id: Uuid,
    pub part_number: String,
    pub part_types_id: i32,
    pub mfg_number: String,
    pub description: Option<String>,
    pub part_mfg_status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusProductModel {
    pub model_code: String,
    pub model_name: String,
    pub description: Option<String>,
}

#[async_trait::async_trait]
pub trait ZeusInventoryClient: Send + Sync {
    async fn get_part(&self, id: Uuid) -> Result<ZeusPart, AppError>;
    async fn create_part(&self, part_catalog_id: Uuid, condition_id: i32, serial_number: &str, mfg_date: chrono::DateTime<chrono::Utc>) -> Result<ZeusPart, AppError>;
    async fn find_product_by_serial(&self, serial_number: &str) -> Result<Option<ZeusProduct>, AppError>;
    async fn get_product(&self, id: Uuid) -> Result<ZeusProduct, AppError>;
    async fn create_product(&self, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError>;
    async fn update_product(&self, id: Uuid, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError>;
    async fn list_products(&self) -> Result<Vec<ZeusProduct>, AppError>;
    async fn find_parts_by_product(&self, product_id: Uuid) -> Result<Vec<ZeusPart>, AppError>;
    async fn get_part_catalog(&self, id: Uuid) -> Result<ZeusPartCatalog, AppError>;
    async fn find_part_catalog_by_part_number(&self, part_number: &str) -> Result<Option<ZeusPartCatalog>, AppError>;
    async fn get_product_model(&self, code: &str) -> Result<ZeusProductModel, AppError>;
}

#[derive(Debug, Clone, Default)]
pub struct MockZeusClient;

#[async_trait::async_trait]
impl ZeusInventoryClient for MockZeusClient {
    async fn get_part(&self, id: Uuid) -> Result<ZeusPart, AppError> {
        Err(AppError::NotFound(format!("Part with ID {} not found", id)))
    }
    async fn create_part(&self, part_catalog_id: Uuid, condition_id: i32, serial_number: &str, mfg_date: chrono::DateTime<chrono::Utc>) -> Result<ZeusPart, AppError> {
        let now = chrono::Utc::now();
        Ok(ZeusPart {
            id: Uuid::new_v4(),
            part_catalog_id,
            part_condition_id: condition_id,
            product_id: None,
            serial_number: serial_number.to_string(),
            manufactured_date: mfg_date,
            installation_date: None,
            removal_date: None,
            scrapped_date: None,
            created_at: now,
            updated_at: now,
        })
    }
    async fn find_product_by_serial(&self, _serial_number: &str) -> Result<Option<ZeusProduct>, AppError> {
        Ok(None)
    }
    async fn get_product(&self, id: Uuid) -> Result<ZeusProduct, AppError> {
        Err(AppError::NotFound(format!("Product with ID {} not found", id)))
    }
    async fn create_product(&self, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError> {
        let now = chrono::Utc::now();
        Ok(ZeusProduct {
            id: Uuid::new_v4(),
            product_model_code: model_code.to_string(),
            customer_id,
            product_name: product_name.to_string(),
            serial_number: serial_number.to_string(),
            created_at: now,
            updated_at: now,
        })
    }
    async fn update_product(&self, id: Uuid, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError> {
        let now = chrono::Utc::now();
        Ok(ZeusProduct {
            id,
            product_model_code: model_code.to_string(),
            customer_id,
            product_name: product_name.to_string(),
            serial_number: serial_number.to_string(),
            created_at: now,
            updated_at: now,
        })
    }
    async fn list_products(&self) -> Result<Vec<ZeusProduct>, AppError> {
        Ok(vec![])
    }
    async fn find_parts_by_product(&self, _product_id: Uuid) -> Result<Vec<ZeusPart>, AppError> {
        Ok(vec![])
    }
    async fn get_part_catalog(&self, id: Uuid) -> Result<ZeusPartCatalog, AppError> {
        Err(AppError::NotFound(format!("Part catalog with ID {} not found", id)))
    }
    async fn find_part_catalog_by_part_number(&self, _part_number: &str) -> Result<Option<ZeusPartCatalog>, AppError> {
        Ok(None)
    }
    async fn get_product_model(&self, code: &str) -> Result<ZeusProductModel, AppError> {
        Err(AppError::NotFound(format!("Product model with code {} not found", code)))
    }
}
