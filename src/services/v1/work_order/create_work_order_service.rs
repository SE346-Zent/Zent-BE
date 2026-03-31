use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

use crate::{
    entities::{work_order_closing_forms, work_orders},
    model::{
        requests::work_order::{
            create_closing_form_request::CreateClosingFormRequest,
            create_work_order_request::CreateWorkOrderRequest,
        },
        responses::{
            error::AppError,
            work_order::{
                closing_form_response::{ClosingFormResponse, ClosingFormResponseData},
                work_order_detail_response::{
                    WorkOrderDetailResponse, WorkOrderDetailResponseData,
                },
            },
        },
    },
};

pub async fn create_work_order_service(
    db: DatabaseConnection,
    request: CreateWorkOrderRequest,
) -> Result<WorkOrderDetailResponse, AppError> {
    let now = Utc::now();

    let appointment = chrono::DateTime::parse_from_rfc3339(&request.appointment)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::BadRequest("Invalid appointment date format. Use RFC3339.".to_string()))?;

    let active_model = work_orders::ActiveModel {
        id: Set(Uuid::new_v4()),
        first_name: Set(request.first_name),
        last_name: Set(request.last_name),
        email: Set(request.email),
        phone_number: Set(request.phone_number),
        work_order_status_id: Set(request.work_order_status_id),
        country: Set(request.country),
        state: Set(request.state),
        city: Set(request.city),
        address: Set(request.address),
        building: Set(request.building),
        appointment: Set(appointment),
        reference_ticket: Set(request.reference_ticket),
        created_at: Set(now),
        updated_at: Set(now),
        closed_at: Set(now),
        admin_id: Set(request.admin_id),
        customer_id: Set(request.customer_id),
        technician_id: Set(request.technician_id),
        complete_form_id: Set(request.complete_form_id),
        reject_reason: Set(request.reject_reason),
    };

    let model = active_model
        .insert(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = WorkOrderDetailResponseData {
        id: model.id,
        first_name: model.first_name,
        last_name: model.last_name,
        email: model.email,
        phone_number: model.phone_number,
        work_order_status_id: model.work_order_status_id,
        country: model.country,
        state: model.state,
        city: model.city,
        address: model.address,
        building: model.building,
        appointment: model.appointment.to_rfc3339(),
        reference_ticket: model.reference_ticket,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        closed_at: model.closed_at.to_rfc3339(),
        admin_id: model.admin_id,
        customer_id: model.customer_id,
        technician_id: model.technician_id,
        complete_form_id: model.complete_form_id,
        reject_reason: model.reject_reason,
    };

    Ok(WorkOrderDetailResponse {
        status_code: 201,
        message: "Work order created successfully".to_string(),
        data,
        meta: None,
    })
}

pub async fn create_closing_form_service(
    db: DatabaseConnection,
    request: CreateClosingFormRequest,
) -> Result<ClosingFormResponse, AppError> {
    let now = Utc::now();

    let active_model = work_order_closing_forms::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_order_counting: Set(request.work_order_counting),
        mtm: Set(request.mtm),
        serial_number: Set(request.serial_number),
        diagnosis: Set(request.diagnosis),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let model = active_model
        .insert(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("DB error: {}", e)))?;

    let data = ClosingFormResponseData {
        id: model.id,
        work_order_counting: model.work_order_counting,
        mtm: model.mtm,
        serial_number: model.serial_number,
        diagnosis: model.diagnosis,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
    };

    Ok(ClosingFormResponse::success(data))
}
