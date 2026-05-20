//! HTTP handlers for media management (photos, signatures, etc.).
//!
//! This module provides endpoints for technicians to upload and update 
//! documentation for work orders, and for retrieving media assets.

pub mod upload_closing_form_photo;
pub mod update_closing_form_photo;
pub mod upload_closing_form_signature;

pub use upload_closing_form_photo::upload_closing_form_photo;
pub use update_closing_form_photo::update_closing_form_photo;
pub use upload_closing_form_signature::upload_closing_form_signature;

// Stub kept for backward compat with integration tests
use axum::http::StatusCode;
pub async fn upload_work_order_photo() -> StatusCode { StatusCode::NOT_IMPLEMENTED }
pub async fn upload_work_order_signature() -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub use upload_closing_form_photo::__path_upload_closing_form_photo;
pub use update_closing_form_photo::__path_update_closing_form_photo;
pub use upload_closing_form_signature::__path_upload_closing_form_signature;

use axum::{Router, middleware, routing};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

/// Initialize and return the Axum router for the media domain.
pub fn media_router(app_state: AppState) -> Router<AppState> {
    let technician_only_closing_routes = Router::new()
        .route("/photos", routing::post(upload_closing_form_photo))
        .route("/photos/{image_id}", routing::patch(update_closing_form_photo))
        .route("/signature", routing::post(upload_closing_form_signature))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    Router::new()
        .nest("/work_orders/{id}/closing_form", technician_only_closing_routes)
        .route("/photos/work_orders/{id}", routing::get(get_work_order_photo))
        .route("/photos/work_orders", routing::get(list_work_order_photos))
        .route_layer(middleware::from_fn_with_state(
            app_state,
            require_role::<AppState>(&[Role::Technician]),
        ))
}

pub async fn get_work_order_photo(
    axum::extract::State(_db_connection): axum::extract::State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }

pub async fn list_work_order_photos(
    axum::extract::State(_db_connection): axum::extract::State<Arc<DatabaseConnection>>,
) -> StatusCode { StatusCode::NOT_IMPLEMENTED }
