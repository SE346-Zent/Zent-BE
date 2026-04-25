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
        },
        pagination::PaginationRequest,
    },
    responses::{
        auth::login_response::LoginResponseData,
        base::MessageOnlyResponse,
        pagination::PaginationResponse,
    },
};

use crate::core::errors::ErrorResponse;

use crate::handlers::v1::auth;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_handler,
        auth::register_handler,
        auth::verify_otp_handler,
        auth::resend_otp_handler,
        auth::refresh_token_handler,
    ),
    components(
        schemas(
            UserLoginRequest,
            UserRegistrationRequest,
            VerifyOtpRequest,
            ResendOtpRequest,
            RefreshTokenRequest,
            LoginResponseData,
            MessageOnlyResponse,
            PaginationRequest,
            PaginationResponse,
            ErrorResponse,
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
