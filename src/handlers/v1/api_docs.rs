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
            change_password_request::ChangePasswordRequest,
        },
        work_orders::{
            create_work_order_request::CreateWorkOrderRequest,
            list_query::WorkOrderQuery,
            start_request::StartWorkOrderRequest,
            approve_refusal_request::ApproveRefusalRequest,
            rate_request::RateWorkOrderRequest,
            reassign_request::ReassignWorkOrderRequest,
            refuse_request::{RefuseWorkOrderRequest, RefuseWorkOrderMultipart},
            change_appointment_request::ChangeAppointmentRequest,
            reject_form_query::RejectFormQuery,
        },
        notifications::{
            list_query::NotificationListQuery,
            update_preference_request::UpdateNotificationPreferenceRequest,
        },
        pagination::PaginationRequest,
        users::{
            profile_update_request::ProfileUpdateRequest,
            user_create_request::UserCreateRequest,
            user_status_update_request::UserStatusUpdateRequest,
        },
    },
    responses::{
        auth::login_response::LoginResponseData,
        auth::verify_forgot_password_otp_response::VerifyForgotPasswordOtpResponseData,
        work_orders::{
            create_response::WorkOrderResponseData,
            list_response::WorkOrderListItem,
            details_response::WorkOrderDetails,
            history_response::{WorkOrderStateHistoryEntry, WorkOrderHistoryDetail, ClosingFormEntry, RatingEntry},
            reject_form_list_response::RejectFormListItem,
            reject_form_detail_response::RejectFormDetail,
        },
        notifications::{
            notification_list_response::NotificationListItem,
            preference_response::NotificationPreferenceResponse,
        },
        base::MessageOnlyResponse,
        pagination::PaginationResponse,
        users::{
            user_response_data::UserResponseData,
            user_list_response_data::UserListResponseData,
            me_response_data::MeResponseData,
        },
    },
};

use crate::core::errors::ErrorResponse;

use crate::handlers::v1::{auth, work_orders, media, inventory, notifications, chat, users};

// API Documentation Service (v1)
//
// This module provides the OpenAPI/utoipa configuration and the Scalar UI
// for interactive API documentation, protected by basic authentication.

#[derive(OpenApi)]
#[openapi(
    paths(
        users::get_me::get_me_handler,
        users::update_me::update_me_handler,
        users::close_account::close_account_handler,
        users::list_users::list_users_handler,
        users::get_user::get_user_handler,
        users::create_user::create_user_handler,
        users::update_user_status::update_user_status_handler,
        auth::login_handler,
        auth::logout_handler,
        auth::register_handler,
        auth::verify_otp_handler,
        auth::resend_otp_handler,
        auth::refresh_token_handler,
        auth::forgot_password_handler,
        auth::verify_forgot_password_otp_handler,
        auth::reset_password_handler,
        auth::change_password_handler,
        work_orders::create,
        work_orders::list,
        work_orders::get_details,
        work_orders::assign,
        work_orders::complete,
        work_orders::refuse,
        work_orders::start,
        inventory::add_parts::add_parts,
        work_orders::approve_refusal,
        work_orders::deny_refusal,
        work_orders::history,
        work_orders::cancel,
        work_orders::rate,
        work_orders::reassign,
        work_orders::change_appointment,
        work_orders::reject_form_list,
        work_orders::reject_form_detail,
        notifications::list::list,
        notifications::get_preferences::get_preferences,
        notifications::update_preferences::update_preferences,
        notifications::unread_count::get_unread_noti_count,
        media::upload_closing_form_photo,
        media::update_closing_form_photo,
        media::upload_closing_form_signature,
        chat::list_rooms::list_rooms,
        chat::get_messages::get_messages,
        chat::upload_attachment::upload_attachment,
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
            ChangePasswordRequest,
            CreateWorkOrderRequest,
            WorkOrderQuery,
            crate::model::requests::work_orders::assign_request::AssignWorkOrderRequest,
            crate::model::requests::work_orders::complete_request::CompleteWorkOrderRequest,
            crate::model::requests::work_orders::cancel_request::CancelWorkOrderRequest,
            RateWorkOrderRequest,
            ReassignWorkOrderRequest,
            ChangeAppointmentRequest,
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
            crate::model::requests::inventory::add_parts_request::AddPartsRequest,
            WorkOrderStateHistoryEntry,
            WorkOrderHistoryDetail,
            ClosingFormEntry,
            RatingEntry,
            RejectFormQuery,
            RejectFormListItem,
            RejectFormDetail,
            NotificationListQuery,
            UpdateNotificationPreferenceRequest,
            NotificationListItem,
            NotificationPreferenceResponse,
            crate::model::responses::chat::room_response::ChatRoomResponse,
            crate::model::responses::chat::message_response::MessageResponse,
            crate::model::requests::chat::list_rooms_query::ListRoomsQuery,
            crate::handlers::v1::chat::upload_attachment::AttachmentUploadResponse,
            ProfileUpdateRequest,
            UserCreateRequest,
            UserStatusUpdateRequest,
            UserResponseData,
            UserListResponseData,
            MeResponseData,
        )
    ),
    modifiers(&SecurityAddon, &EndpointPathTitles),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "work_orders", description = "Work order management"),
        (name = "inventory", description = "Inventory management"),
        (name = "notifications", description = "Notification management"),
        (name = "media", description = "Media/OCI endpoints"),
        (name = "chat", description = "Chat & messaging endpoints"),
        (name = "users", description = "User management endpoints"),
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

/// Sets every operation's summary to `{METHOD} {path}` so Scalar tabs
/// show short endpoint identifiers instead of verbose Rust doc comments.
struct EndpointPathTitles;

impl Modify for EndpointPathTitles {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        for (path, item) in openapi.paths.paths.iter_mut() {
            if let Some(op) = &mut item.get {
                op.summary = Some(format!("GET {}", path));
            }
            if let Some(op) = &mut item.post {
                op.summary = Some(format!("POST {}", path));
            }
            if let Some(op) = &mut item.put {
                op.summary = Some(format!("PUT {}", path));
            }
            if let Some(op) = &mut item.patch {
                op.summary = Some(format!("PATCH {}", path));
            }
            if let Some(op) = &mut item.delete {
                op.summary = Some(format!("DELETE {}", path));
            }
        }
    }
}

/// Middleware to enforce basic authentication for the interactive API documentation UI.

async fn check_docs_auth(
    request: Request<axum::body::Body>,
    next_middleware_service: Next,
) -> Result<Response, impl IntoResponse> {
    let config = AppConfig::get();
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth) = auth_header {
        if let Some(encoded) = auth.strip_prefix("Basic ") {
            if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(encoded) {
                if let Ok(credentials) = String::from_utf8(decoded) {
                    let parts: Vec<&str> = credentials.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let is_username_valid = parts[0].as_bytes().ct_eq(config.docs_username.as_bytes());
                        let is_password_valid = parts[1].as_bytes().ct_eq(config.docs_password.as_bytes());
                        if is_username_valid.unwrap_u8() == 1 && is_password_valid.unwrap_u8() == 1 {
                            return Ok(next_middleware_service.run(request).await);
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

/// Initialize the documentation router, mounting the Scalar UI with auth protection.

pub fn router() -> axum::Router<crate::core::state::AppState> {
    axum::Router::new()
        .merge(Scalar::with_url("/scalar", ApiDoc::openapi()))
        .route_layer(middleware::from_fn(check_docs_auth))
}
