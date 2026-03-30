use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable};

use crate::model::{
    requests::{
        account::profile_list_query::RoleQuery,
        auth::{user_login_request::UserLoginRequest, user_registration_request::UserRegistrationRequest},
        work_order::create_work_order_request::CreateWorkOrderRequest,
        work_order::create_closing_form_request::CreateClosingFormRequest,
    },
    responses::{
        account::profile_detail_response::{ProfileDetailResponse, ProfileDetailResponseData},
        account::profile_list_item_response::{ProfileListResponse, ProfileListItemResponseData},
        auth::{login_response::LoginResponse, login_response::LoginResponseData, register_response::RegisterResponse},
        common::pagination_meta::PaginationMeta,
        error::ErrorResponse,
        work_order::work_order_detail_response::{WorkOrderDetailResponse, WorkOrderDetailResponseData},
        work_order::work_order_list_item_response::{WorkOrderListResponse, WorkOrderListItemResponseData},
        work_order::closing_form_response::{ClosingFormResponse, ClosingFormResponseData},
        warranty::warranty_detail_response::{WarrantyDetailResponse, WarrantyDetailResponseData},
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
        work_order::create_work_order,
        work_order::create_closing_form,
        work_order::get_warranty,
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
            ProfileListItemResponseData,
            ProfileDetailResponse,
            ProfileDetailResponseData,
            WorkOrderListResponse,
            WorkOrderListItemResponseData,
            WorkOrderDetailResponse,
            WorkOrderDetailResponseData,
            CreateWorkOrderRequest,
            CreateClosingFormRequest,
            ClosingFormResponse,
            ClosingFormResponseData,
            WarrantyDetailResponse,
            WarrantyDetailResponseData,
            PaginationMeta,
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

pub fn router() -> axum::Router<crate::state::AppState> {
    axum::Router::new().merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
}
