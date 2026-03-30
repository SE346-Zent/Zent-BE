use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct WarrantyDetailQuery {
    #[serde(rename = "warrantyId")]
    pub warranty_id: Uuid,
}
