use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::register_device_request::RegisterDeviceRequest;
use crate::model::responses::inventory::register_device_response::RegisterDeviceResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::register_device::decide_register_device;
use crate::services::v1::core::email_service;
use crate::entities::{registered_devices, warranties};
use chrono::Utc;
use sea_orm::{EntityTrait, ActiveModelTrait, QueryFilter, ColumnTrait, Set};
use validator::Validate;

/// Register a new device by a customer with warranty check, Zeus sync, and optional email confirmation.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/devices/register",
    tag = "inventory",
    request_body = RegisterDeviceRequest,
    responses(
        (status = 200, description = "Device registered successfully", body = ApiResponse<RegisterDeviceResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn register_device(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<ApiResponse<RegisterDeviceResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Find product in Zeus SCM by serial number
    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?;

    let (product_model_code, model_name, existing_product_id) = match &zeus_prod {
        Some(p) => {
            let model_def = state.zeus_client.get_product_model(&p.product_model_code).await.ok();
            (
                p.product_model_code.clone(),
                model_def.map(|m| m.model_name).unwrap_or(p.product_name.clone()),
                Some(p.id),
            )
        }
        None => {
            return Err(AppError::BadRequest(format!("Serial number '{}' not found in product catalog", payload.serial_number)));
        }
    };

    let zeus_prod = zeus_prod.unwrap();

    // Check if device is already registered by this customer
    let existing_registration = registered_devices::Entity::find()
        .filter(registered_devices::Column::CustomerId.eq(auth.user.id))
        .filter(registered_devices::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?;

    if existing_registration.is_some() {
        return Err(AppError::BadRequest("You have already registered this device".to_string()));
    }

    let effect = decide_register_device(
        &payload,
        auth.user.id,
        &auth.user.full_name,
        zeus_prod.id,
        product_model_code,
        model_name,
        Utc::now(),
    )?;

    // Sync with Zeus - update product ownership if needed
    if zeus_prod.customer_id != auth.user.id {
        state.zeus_client.update_product(
            zeus_prod.id,
            &zeus_prod.product_model_code,
            auth.user.id,
            &zeus_prod.product_name,
            &zeus_prod.serial_number,
        ).await?;
    }

    // Check warranty status
    let existing_warranty = warranties::Entity::find()
        .filter(warranties::Column::ProductId.eq(zeus_prod.id))
        .one(state.db.as_ref())
        .await?;

    let warranty_status = match existing_warranty {
        Some(w) => {
            let now = Utc::now();
            if now > w.end_date {
                "expired".to_string()
            } else {
                w.warranty_status.clone()
            }
        }
        None => "none".to_string(),
    };

    // Create registration record
    let registration_model = registered_devices::ActiveModel {
        id: Set(effect.registration_id),
        customer_id: Set(effect.customer_id),
        product_id: Set(effect.product_id),
        country: Set(effect.country.clone()),
        province: Set(effect.province.clone()),
        ward: Set(effect.ward.clone()),
        address: Set(effect.address.clone()),
        first_name: Set(effect.first_name.clone()),
        last_name: Set(effect.last_name.clone()),
        email: Set(effect.email.clone()),
        mobile_phone: Set(effect.mobile_phone.clone()),
        email_confirmation_sent: Set(effect.should_send_confirmation_email),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        deleted_at: Set(None),
    };

    registration_model.insert(state.db.as_ref()).await?;

    // Send confirmation email if requested via email service
    if effect.should_send_confirmation_email {
        if let Some(ref conn) = state.rabbitmq {
            let full_address = format!("{}, {}, {}", effect.address, effect.ward, effect.province);
            let registration_date = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();

            let _ = email_service::send_device_registration_email(
                conn,
                &state.templates,
                &effect.email,
                &effect.customer_full_name,
                &effect.product_display_name,
                &effect.product_serial_number,
                &effect.country,
                &effect.province,
                &full_address,
                &warranty_status,
                &registration_date,
            ).await;
        }
    }

    Ok(Json(ApiResponse::success(
        200,
        "Device registered successfully",
        RegisterDeviceResponse {
            registration_id: effect.registration_id,
            product_id: effect.product_id,
            serial_number: effect.product_serial_number,
            message: "Device registered successfully".to_string(),
            email_sent: effect.should_send_confirmation_email,
            warranty_status,
        },
    )))
}
