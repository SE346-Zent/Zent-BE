use crate::core::errors::AppError;
use crate::model::requests::inventory::list_parts_query::ListPartsQuery;
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
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
pub struct ZeusPartList {
    pub items: Vec<ZeusPart>,
    pub total_rows: u64,
    pub total_pages: u64,
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
pub struct ZeusProductList {
    pub items: Vec<ZeusProduct>,
    pub total_rows: u64,
    pub total_pages: u64,
}

#[async_trait::async_trait]
pub trait ZeusInventoryClient: Send + Sync {
    async fn list_parts(&self, query: &ListPartsQuery) -> Result<ZeusPartList, AppError>;
    async fn get_part(&self, id: Uuid) -> Result<ZeusPart, AppError>;
    async fn create_part(&self, part_catalog_id: Uuid, condition_id: i32, serial_number: &str, mfg_date: chrono::DateTime<chrono::Utc>) -> Result<ZeusPart, AppError>;
    async fn list_products(&self, query: &ListProductsQuery) -> Result<ZeusProductList, AppError>;
    async fn get_product(&self, id: Uuid) -> Result<ZeusProduct, AppError>;
    async fn create_product(&self, model_code: &str, customer_id: Uuid, product_name: &str, serial_number: &str) -> Result<ZeusProduct, AppError>;
}

#[derive(Debug, Clone, Default)]
pub struct MockZeusClient;

#[async_trait::async_trait]
impl ZeusInventoryClient for MockZeusClient {
    async fn list_parts(&self, _query: &ListPartsQuery) -> Result<ZeusPartList, AppError> {
        Ok(ZeusPartList { items: vec![], total_rows: 0, total_pages: 0 })
    }
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
    async fn list_products(&self, _query: &ListProductsQuery) -> Result<ZeusProductList, AppError> {
        Ok(ZeusProductList { items: vec![], total_rows: 0, total_pages: 0 })
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
}
