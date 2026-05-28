use axum::{extract::State, Json};
use crate::core::state::AppState;
use crate::core::errors::{AppError, ErrorResponse};
use crate::extractor::auth_user::AuthUser;
use crate::model::requests::inventory::register_product_request::RegisterProductRequest;
use crate::model::responses::inventory::register_product_response::RegisterProductResponse;
use crate::model::responses::base::ApiResponse;
use crate::services::v1::inventory::register_product::{self, decide_register_product};
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

    let zeus_prod = state.zeus_client.find_product_by_serial(&payload.serial_number).await?;

    let (product_model_code, model_name) = match &zeus_prod {
        Some(p) => {
            let model_def = state.zeus_client.get_product_model(&p.product_model_code).await.ok();
            (
                Some(p.product_model_code.clone()),
                Some(model_def.map(|m| m.model_name).unwrap_or(p.product_name.clone())),
            )
        }
        None => (None, None),
    };

    let existing_id = zeus_prod.as_ref().map(|p| p.id);

    let effect = decide_register_product(
        &payload,
        auth.user.id,
        &auth.user.full_name,
        product_model_code,
        model_name,
        existing_id,
        Utc::now(),
    )?;

    let product_id = if let Some(existing_prod) = zeus_prod {
        let synced = if existing_prod.customer_id != auth.user.id {
            state
                .zeus_client
                .update_product(
                    existing_prod.id,
                    &existing_prod.product_model_code,
                    auth.user.id,
                    &existing_prod.product_name,
                    &existing_prod.serial_number,
                )
                .await?
        } else {
            existing_prod
        };
        synced.id
    } else {
        let zeus_registered = state
            .zeus_client
            .create_product(
                &effect.product_model_code,
                effect.customer_id,
                &effect.product_display_name,
                &effect.product_serial_number,
            )
            .await?;
        zeus_registered.id
    };

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
