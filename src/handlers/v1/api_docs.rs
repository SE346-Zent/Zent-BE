use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable};
use axum::{
    middleware::{self, Next},
    response::{Response, IntoResponse},
    http::{Request, StatusCode, header},
};
use base64::Engine;
use subtle::ConstantTimeEq;
use crate::core::config::AppConfig;

use crate::model::{
    requests::{
        auth::{
            user_login_request::UserLoginRequest, 
            user_registration_request::UserRegistrationRequest,
            verify_otp_request::VerifyOtpRequest,
            resend_otp_request::ResendOtpRequest,
            refresh_token_request::RefreshTokenRequest,
            forgot_password_request::ForgotPasswordRequest,
            verify_forgot_password_otp_request::VerifyForgotPasswordOtpRequest,
            reset_password_request::ResetPasswordRequest,
            logout_request::LogoutRequest,
        },
        work_orders::{
            create_work_order_request::CreateWorkOrderRequest,
            list_query::WorkOrderQuery,
            start_request::StartWorkOrderRequest,
            approve_refusal_request::ApproveRefusalRequest,
            add_parts_request::AddPartsRequest,
            refuse_request::{RefuseWorkOrderRequest, RefuseWorkOrderMultipart},

        },
        pagination::PaginationRequest,
    },
    responses::{
        auth::login_response::LoginResponseData,
        auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
        work_orders::{
            create_response::WorkOrderResponseData,
            list_response::WorkOrderListItem,
            details_response::WorkOrderDetails,
            history_response::WorkOrderStateHistoryEntry,
        },
        base::MessageOnlyResponse,
        pagination::PaginationResponse,
    },
};

use crate::core::errors::ErrorResponse;

use crate::handlers::v1::{auth, work_orders, media};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_handler,
        auth::logout_handler,
        auth::register_handler,
        auth::verify_otp_handler,
        auth::resend_otp_handler,
        auth::refresh_token_handler,
        auth::forgot_password_handler,
        auth::verify_forgot_password_otp_handler,
        auth::reset_password_handler,
        work_orders::create,
        work_orders::list,
        work_orders::get_details,
        work_orders::assign,
        work_orders::complete,
        work_orders::refuse,
        work_orders::start,
        work_orders::add_parts,
        work_orders::approve_refusal,
        work_orders::deny_refusal,
        work_orders::history,
        media::upload_new_part_photo,
        media::upload_closing_form_photo,
        media::update_closing_form_photo,
        media::upload_closing_form_signature,
    ),
    components(
        schemas(
            UserLoginRequest,
            UserRegistrationRequest,
            VerifyOtpRequest,
            ResendOtpRequest,
            RefreshTokenRequest,
            ForgotPasswordRequest,
            VerifyForgotPasswordOtpRequest,
            ResetPasswordRequest,
            LogoutRequest,
            CreateWorkOrderRequest,
            WorkOrderQuery,
            crate::model::requests::work_orders::assign_request::AssignWorkOrderRequest,
            crate::model::requests::work_orders::complete_request::CompleteWorkOrderRequest,
            crate::model::requests::work_orders::complete_request::PartChangeInput,
            LoginResponseData,
            VerifyForgotPasswordOtpResponseData,
            WorkOrderResponseData,
            WorkOrderListItem,
            WorkOrderDetails,
            MessageOnlyResponse,
            PaginationRequest,
            PaginationResponse,
            ErrorResponse,
            RefuseWorkOrderRequest,
            RefuseWorkOrderMultipart,
            StartWorkOrderRequest,
            ApproveRefusalRequest,
            AddPartsRequest,
            WorkOrderStateHistoryEntry,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Zent-BE", description = "Zent Backend API endpoints")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

async fn check_docs_auth(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let config = AppConfig::get();
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth) = auth_header {
        if auth.starts_with("Basic ") {
            let encoded = &auth[6..];
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let user_ok = parts[0].as_bytes().ct_eq(config.docs_username.as_bytes());
                        let pass_ok = parts[1].as_bytes().ct_eq(config.docs_password.as_bytes());
                        if user_ok.unwrap_u8() == 1 && pass_ok.unwrap_u8() == 1 {
                            return Ok(next.run(req).await);
                        }
                    }
                }
            }
        }
    }

    let response = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"Zent API Documentation\"")
        .body(axum::body::Body::empty())
        .unwrap();

    Err(response)
}

pub fn router() -> axum::Router<crate::core::state::AppState> {
    axum::Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route_layer(middleware::from_fn(check_docs_auth))
}
