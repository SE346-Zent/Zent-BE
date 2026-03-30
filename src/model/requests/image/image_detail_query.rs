use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ImageDetailQuery {
    #[serde(rename = "imageId")]
    pub image_id: Uuid,
}
