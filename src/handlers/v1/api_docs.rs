use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable};

use crate::model::{
    requests::{
        account::profile_list_query::RoleQuery,
        auth::{user_login_request::UserLoginRequest, user_registration_request::UserRegistrationRequest},
    },
    responses::{
        account::profile_response::{ProfileListResponse, ProfileResponse, ProfileResponseData},
        auth::{login_response::LoginResponse, login_response::LoginResponseData, register_response::RegisterResponse},
        common::pagination::PaginationMeta,
        work_order::work_order_response::{WorkOrderListResponse, WorkOrderResponse, WorkOrderResponseData},
    },
};

use crate::handlers::v1::{account, auth, work_order};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_handler,
        auth::register_handler,
        account::get_profile,
        account::get_profiles,
        work_order::get_my_work_order,
        work_order::get_my_work_orders,
    ),
    components(
        schemas(
            UserLoginRequest,
            UserRegistrationRequest,
            LoginResponse,
            LoginResponseData,
            RegisterResponse,
            RoleQuery,
            ProfileListResponse,
            ProfileResponse,
            ProfileResponseData,
            WorkOrderListResponse,
            WorkOrderResponse,
            WorkOrderResponseData,
            PaginationMeta,
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

pub fn router() -> axum::Router<crate::state::AppState> {
    axum::Router::new().merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
}
