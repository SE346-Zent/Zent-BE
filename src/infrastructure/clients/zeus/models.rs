use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct ZeusEnvelope<T> {
    pub data: Option<T>,
    #[allow(non_snake_case)]
    pub statusCode: u16,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ZeusPartDto {
    #[serde(rename = "ID")]
    pub id: Uuid,
    #[serde(rename = "PartCatalogID")]
    pub part_catalog_id: Uuid,
    #[serde(rename = "PartConditionID")]
    pub part_condition_id: i32,
    #[serde(rename = "ProductID")]
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
#[serde(rename_all = "PascalCase")]
pub(crate) struct ZeusProductDto {
    #[serde(rename = "ID")]
    pub id: Uuid,
    pub product_model_code: String,
    #[serde(rename = "CustomerID")]
    pub customer_id: Uuid,
    pub product_name: String,
    pub serial_number: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
