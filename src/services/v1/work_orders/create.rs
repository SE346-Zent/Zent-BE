use sea_orm::Set;
use crate::{
    core::errors::AppError,
    entities::work_orders,
    model::requests::work_orders::create_work_order_request::CreateWorkOrderRequest,
};
use uuid::Uuid;
use chrono::Utc;

pub struct CreateWorkOrderEffect {
    pub work_order: work_orders::ActiveModel,
}

pub fn decide_create_work_order(
    req: CreateWorkOrderRequest,
    customer_id: Uuid,
    pending_status_id: i32,
) -> Result<CreateWorkOrderEffect, AppError> {
    // 1. Location Policy Validation
    if req.city != "HCM" || req.province != "HN" {
        return Err(AppError::BadRequest("Only HCM and HN are supported at this time".to_string()));
    }

    // 2. ID and Number Generation
    let now = Utc::now();
    let wo_id = Uuid::new_v4();
    let work_order_number = format!("WO-{}", &wo_id.to_string()[..6].to_uppercase());

    
    let work_order = work_orders::ActiveModel {
        id: Set(wo_id),
        work_order_status_id: Set(pending_status_id),
        customer_id: Set(customer_id),
        product_id: Set(req.product_id),
        reference_ticket_id: Set(req.reference_ticket_id),
        work_order_symptom_id: Set(req.work_order_symptom_id),
        description: Set(req.description),
        first_name: Set(req.first_name),
        last_name: Set(req.last_name),
        email: Set(req.email),
        phone_number: Set(req.phone_number),
        country: Set(req.country),
        province: Set(req.province),
        city: Set(req.city),
        address: Set(req.address),
        building: Set(req.building),
        appointment: Set(req.appointment),
        work_order_number: Set(work_order_number),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    Ok(CreateWorkOrderEffect { work_order })
}
