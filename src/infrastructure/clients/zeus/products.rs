use crate::services::v1::inventory::ports::ZeusProduct;
use serde::Serialize;
use uuid::Uuid;

use super::models::ZeusProductDto;

pub(crate) struct ProductsApi;

impl ProductsApi {
    pub fn to_domain(dto: ZeusProductDto) -> ZeusProduct {
        ZeusProduct {
            id: dto.id,
            product_model_code: dto.product_model_code,
            customer_id: dto.customer_id,
            product_name: dto.product_name,
            serial_number: dto.serial_number,
            created_at: dto.created_at,
            updated_at: dto.updated_at,
        }
    }

    pub fn create_product_payload(
        product_model_code: &str,
        customer_id: Uuid,
        product_name: &str,
        serial_number: &str,
    ) -> CreateProductPayload {
        CreateProductPayload {
            product_model_code: product_model_code.to_string(),
            customer_id,
            product_name: product_name.to_string(),
            serial_number: serial_number.to_string(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct CreateProductPayload {
    pub product_model_code: String,
    pub customer_id: Uuid,
    pub product_name: String,
    pub serial_number: String,
}
