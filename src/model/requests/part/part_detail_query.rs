use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize, Serialize, ToSchema, Debug)]
pub struct PartDetailQuery {
    #[serde(rename = "partId")]
    pub part_id: Uuid,
}
