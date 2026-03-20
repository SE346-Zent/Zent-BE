use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, Debug, IntoParams)]
pub struct WorkOrderQuery {
    #[serde(rename = "Id")]
    pub id: Uuid,
}
