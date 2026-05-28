//! HTTP handlers for inventory and product management.
//!
//! This module provides the REST API endpoints for managing the product catalog,
//! registering new parts, and overseeing the part approval workflow.

pub mod add_parts;
pub mod get_part;
pub mod get_product;
pub mod check_serial;
pub mod check_warranty;
pub mod register_product;
pub mod accept_part;
pub mod deny_part;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

/// Initialize and return the Axum router for the inventory domain.
pub fn router(app_state: AppState) -> Router<AppState> {
    let technician_only_routes = Router::new()
        .route("/work_orders/{id}/parts", axum::routing::post(add_parts::add_parts))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    let administrator_only_routes = Router::new()
        .route("/parts/{id}/accept", axum::routing::post(accept_part::accept_part))
        .route("/parts/{id}/deny", axum::routing::post(deny_part::deny_part))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    Router::new()
        .route("/parts/{id}", axum::routing::get(get_part::get_part))
        .route("/products/{id}", axum::routing::get(get_product::get_product))
        .route("/products/check-serial", axum::routing::post(check_serial::check_serial))
        .route("/products/check-warranty", axum::routing::post(check_warranty::check_warranty))
        .route("/products/register", axum::routing::post(register_product::register_product))
        .merge(technician_only_routes)
        .merge(administrator_only_routes)
}
