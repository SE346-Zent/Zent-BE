use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable};

use crate::model::{
    requests::{
        auth::{user_login_request::UserLoginRequest, user_registration_request::UserRegistrationRequest},
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
    ),
    components(
        schemas(
            UserLoginRequest,
            UserRegistrationRequest,
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

pub fn router() -> axum::Router<crate::core::state::AppState> {
    axum::Router::new().merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
}
