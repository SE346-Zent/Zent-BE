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
    #[serde(alias = "SerialNumber", alias = "serial_number")]
    pub serial_number: String,
    #[serde(alias = "ManufacturedDate", alias = "manufactured_date")]
    pub manufactured_date: chrono::DateTime<chrono::Utc>,
    #[serde(alias = "InstallationDate", alias = "installation_date")]
    pub installation_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(alias = "RemovalDate", alias = "removal_date")]
    pub removal_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(alias = "ScrappedDate", alias = "scrapped_date")]
    pub scrapped_date: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(alias = "CreatedAt", alias = "created_at")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(alias = "UpdatedAt", alias = "updated_at")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusProductDto {
    #[serde(rename = "id", alias = "ID")]
    pub id: Uuid,
    #[serde(alias = "ProductModelCode", alias = "product_model_code")]
    pub product_model_code: String,
    #[serde(rename = "customer_id", alias = "CustomerID")]
    pub customer_id: Uuid,
    #[serde(alias = "ProductName", alias = "product_name")]
    pub product_name: String,
    #[serde(alias = "SerialNumber", alias = "serial_number")]
    pub serial_number: String,
    #[serde(alias = "CreatedAt", alias = "created_at")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(alias = "UpdatedAt", alias = "updated_at")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ZeusPartCatalogDto {
    #[serde(rename = "id", alias = "ID")]
    pub id: Uuid,
    #[serde(alias = "PartNumber", alias = "part_number")]
    pub part_number: String,
    #[serde(alias = "PartTypesID", alias = "part_types_id")]
    pub part_types_id: i32,
    #[serde(alias = "MfgNumber", alias = "mfg_number")]
    pub mfg_number: String,
    #[serde(alias = "Description", alias = "description")]
    pub description: Option<String>,
    #[serde(alias = "PartMfgStatus", alias = "part_mfg_status")]
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
