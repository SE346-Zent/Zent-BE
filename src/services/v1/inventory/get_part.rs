use crate::core::errors::AppError;
use crate::entities::{parts, part_catalog, part_conditions, products};
use crate::model::responses::inventory::part_detail_response::PartDetailResponse;

pub struct PartWithRelations {
    pub part: parts::Model,
    pub catalog: part_catalog::Model,
    pub condition: part_conditions::Model,
    pub product: Option<products::Model>,
    pub status: String,
    pub denial_reason: Option<String>,
    pub customer_id: Option<uuid::Uuid>,
    pub technician_id: Option<uuid::Uuid>,
}

fn can_user_see(role_name: &str, user_id: uuid::Uuid, p: &PartWithRelations) -> bool {
    match role_name.to_lowercase().as_str() {
        "admin" | "manager" => true,
        "technician" => p.technician_id == Some(user_id),
        "customer" => p.status == "approved" && p.customer_id == Some(user_id),
        _ => false,
    }
}

pub fn get_part_detail(
    p: &PartWithRelations,
    role_name: &str,
    user_id: uuid::Uuid,
) -> Result<PartDetailResponse, AppError> {
    if !can_user_see(role_name, user_id, p) {
        return Err(AppError::Forbidden("You do not have access to this part".to_string()));
    }
    Ok(PartDetailResponse {
        part_id: p.part.id,
        part_number: p.catalog.part_number.clone(),
        part_type_id: p.catalog.part_types_id,
        part_type_name: p.catalog.part_types_id.to_string(),
        model_code: None,
        serial_number: p.part.serial_number.clone(),
        description: p.catalog.description.clone(),
        condition_id: p.condition.id,
        condition_name: p.condition.name.clone(),
        product_id: p.product.as_ref().map(|x| x.id),
        product_name: p.product.as_ref().map(|x| x.product_name.clone()),
        manufactured_date: Some(p.part.manufactured_date.to_rfc3339()),
        installation_date: p.part.installation_date.map(|d| d.to_rfc3339()),
        approval_status: p.status.clone(),
        denial_reason: p.denial_reason.clone(),
        created_at: p.part.created_at.to_rfc3339(),
        updated_at: p.part.updated_at.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::TimeZone;

    fn u(s: &str) -> Uuid { Uuid::parse_str(s).unwrap() }
    fn t() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.from_utc_datetime(&chrono::NaiveDateTime::parse_from_str("2024-01-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap())
    }

    #[test]
    fn test_admin_can_get_detail() {
        let p = PartWithRelations {
            part: parts::Model { id: u("10000000-0000-0000-0000-000000000000"), part_catalog_id: u("00000000-0000-0000-0000-000000000000"), product_id: None, serial_number: "SN-1".into(), part_condition_id: 1, manufactured_date: t(), installation_date: None, removal_date: None, scrapped_date: None, created_at: t(), updated_at: t(), deleted_at: None },
            catalog: part_catalog::Model { id: u("00000000-0000-0000-0000-000000000000"), part_number: "PN-001".into(), part_types_id: 1, mfg_number: "MFG".into(), description: None, part_mfg_status: 1, created_at: t(), updated_at: t(), deleted_at: None },
            condition: part_conditions::Model { id: 1, name: "New".into() },
            product: None, status: "pending".into(), denial_reason: None,
            customer_id: None, technician_id: Some(u("b0000000-0000-0000-0000-000000000000")),
        };
        let r = get_part_detail(&p, "Admin", u("a0000000-0000-0000-0000-000000000000"));
        assert!(r.is_ok());
    }
}
