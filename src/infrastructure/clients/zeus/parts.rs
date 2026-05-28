use crate::services::v1::inventory::ports::ZeusPart;
use serde::Serialize;
use uuid::Uuid;

use super::models::ZeusPartDto;

pub(crate) struct PartsApi;

impl PartsApi {
    pub fn to_domain(dto: ZeusPartDto) -> ZeusPart {
        ZeusPart {
            id: dto.id,
            part_catalog_id: dto.part_catalog_id,
            part_condition_id: dto.part_condition_id,
            product_id: dto.product_id,
            serial_number: dto.serial_number,
            manufactured_date: dto.manufactured_date,
            installation_date: dto.installation_date,
            removal_date: dto.removal_date,
            scrapped_date: dto.scrapped_date,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }

    pub fn create_part_payload(
        part_catalog_id: Uuid,
        condition_id: i32,
        serial_number: &str,
        manufactured_date: chrono::DateTime<chrono::Utc>,
    ) -> CreatePartPayload {
        CreatePartPayload {
            part_catalog_id,
            part_condition_id: condition_id,
            serial_number: serial_number.to_string(),
            manufactured_date,
        }
    }

    pub fn install_part_payload(product_id: Uuid) -> InstallPartPayload {
        InstallPartPayload { product_id }
    }
}

#[derive(Serialize)]
pub(crate) struct CreatePartPayload {
    pub part_catalog_id: Uuid,
    pub part_condition_id: i32,
    pub serial_number: String,
    pub manufactured_date: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub(crate) struct InstallPartPayload {
    pub product_id: Uuid,
}
