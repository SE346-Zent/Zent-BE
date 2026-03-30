use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams, Debug)]
pub struct EquipmentDetailQuery {
    #[serde(rename = "equipmentId")]
    pub equipment_id: Uuid,
}
