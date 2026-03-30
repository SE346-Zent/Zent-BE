use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug, IntoParams)]
pub struct WarrantyDetailQuery {
    #[serde(rename = "warrantyId")]
    pub warranty_id: Uuid,
}
