use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::register_product_request::RegisterProductRequest;
use crate::model::requests::inventory::list_products_query::ListProductsQuery;
use crate::model::responses::inventory::register_product_response::RegisterProductResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::register_product::{self, decide_register_product};
use crate::entities::{products as prod, product_models};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use chrono::Utc;
use validator::Validate;

/// Register a new product by a customer and synchronize it with Zeus SCM.
#[utoipa::path(
    post,
    path = "/api/v1/inventory/products/register",
    request_body = RegisterProductRequest,
    responses(
        (status = 200, description = "Product registered successfully", body = ApiResponse<RegisterProductResponse>),
        (status = 400, description = "Bad Request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn register_product(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<RegisterProductRequest>,
) -> Result<Json<ApiResponse<RegisterProductResponse>>, AppError> {
    payload.validate().map_err(|e| AppError::BadRequest(e.to_string()))?;

    // Query Zeus to verify the serial number exists in the product catalog
    let query = ListProductsQuery {
        search: Some(payload.serial_number.clone()),
        ..Default::default()
    };
    let zeus_list = state.zeus_client.list_products(&query).await?;
    let zeus_prod = zeus_list.items.into_iter().find(|p| p.serial_number == payload.serial_number)
        .ok_or_else(|| AppError::BadRequest(format!("Serial number '{}' not found in product catalog", payload.serial_number)))?;

    // Retrieve product model definition from local database
    let model_def = product_models::Entity::find_by_id(zeus_prod.product_model_code.clone())
        .one(state.db.as_ref())
        .await?;

    // Query local database for existing registered products with the same serial number
    let existing = prod::Entity::find()
        .filter(prod::Column::SerialNumber.eq(&payload.serial_number))
        .one(state.db.as_ref())
        .await?;
    let existing_id = existing.map(|p| p.id);

    let effect = decide_register_product(
        &payload,
        auth.user.id,
        &auth.user.full_name,
        Some(zeus_prod.product_model_code.clone()),
        Some(model_def.map(|m| m.model_name).unwrap_or(zeus_prod.product_name)),
        existing_id,
        Utc::now(),
    )?;

    let product_id = if existing_id.is_none() {
        // Register the product on Zeus SCM API
        let zeus_registered = state.zeus_client.create_product(
            &effect.product_model_code,
            effect.customer_id,
            &effect.product_display_name,
            &effect.product_serial_number,
        ).await?;

        // Save registration record locally in Zent DB
        let new_db_product = prod::ActiveModel {
            id: Set(zeus_registered.id),
            product_model_code: Set(zeus_registered.product_model_code),
            customer_id: Set(zeus_registered.customer_id),
            product_name: Set(zeus_registered.product_name),
            serial_number: Set(zeus_registered.serial_number),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            deleted_at: Set(None),
        };
        new_db_product.insert(state.db.as_ref()).await?;
        zeus_registered.id
    } else {
        effect.registered_product_id
    };

    // If needed, send confirmation email via RabbitMQ
    if effect.should_send_confirmation_email {
        if let Some(ref conn) = state.rabbitmq {
            let email_payload = serde_json::json!({
                "to": effect.customer_email_address,
                "subject": "Product Registration Confirmation",
                "body": format!("Hello {}, your product {} (Serial: {}) has been successfully registered.", effect.customer_full_name, effect.product_display_name, effect.product_serial_number)
            });
            let _ = crate::services::v1::core::helpers::mq::publish_email_task(
                conn,
                email_payload,
                "register product confirmation",
            ).await;
        }
    }

    Ok(Json(ApiResponse::success(
        200,
        "Product registered successfully",
        RegisterProductResponse {
            product_id,
            serial_number: effect.product_serial_number,
            message: "Product registered successfully".to_string(),
            email_sent: effect.should_send_confirmation_email,
        },
    )))
}
