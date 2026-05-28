use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct ZeusEnvelope<T> {
    pub data: Option<T>,
    #[serde(rename = "statusCode", alias = "status_code")]
    pub statusCode: u16,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusPartDto {
    #[serde(rename = "id", alias = "ID")]
    pub id: Uuid,
    #[serde(rename = "part_catalog_id", alias = "PartCatalogID")]
    pub part_catalog_id: Uuid,
    #[serde(rename = "part_condition_id", alias = "PartConditionID")]
    pub part_condition_id: i32,
    #[serde(rename = "product_id", alias = "ProductID")]
    pub product_id: Option<Uuid>,
    pub serial_number: String,
    pub manufactured_date: chrono::DateTime<chrono::Utc>,
    pub installation_date: Option<chrono::DateTime<chrono::Utc>>,
    pub removal_date: Option<chrono::DateTime<chrono::Utc>>,
    pub scrapped_date: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusProductDto {
    #[serde(rename = "id", alias = "ID")]
    pub id: Uuid,
    pub product_model_code: String,
    #[serde(rename = "customer_id", alias = "CustomerID")]
    pub customer_id: Uuid,
    pub product_name: String,
    pub serial_number: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusPartCatalogDto {
    #[serde(rename = "id", alias = "ID")]
    pub id: Uuid,
    pub part_number: String,
    pub part_types_id: i32,
    pub mfg_number: String,
    pub description: Option<String>,
    pub part_mfg_status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusProductModelDto {
    pub model_code: String,
    pub model_name: String,
    pub description: Option<String>,
    #[serde(rename = "image_url", alias = "imageUrl", alias = "object_name")]
    pub image_url: Option<String>,
}
