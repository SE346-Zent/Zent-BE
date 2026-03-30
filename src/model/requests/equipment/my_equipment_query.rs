use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

#[derive(Deserialize, IntoParams, Debug)]
pub struct MyEquipmentQuery {
    pub customer_id: Uuid,
}