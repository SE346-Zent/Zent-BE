pub mod add_parts;
pub mod list_parts;
pub mod get_part;
pub mod list_products;
pub mod get_product;
pub mod check_serial;
pub mod register_product;
pub mod accept_part;
pub mod deny_part;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub fn router(state: AppState) -> Router<AppState> {
    let tech_routes = Router::new()
        .route("/work_orders/{id}/parts", axum::routing::post(add_parts::add_parts))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    let admin_routes = Router::new()
        .route("/parts/{id}/accept", axum::routing::post(accept_part::accept_part))
        .route("/parts/{id}/deny", axum::routing::post(deny_part::deny_part))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    Router::new()
        .route("/parts", axum::routing::get(list_parts::list_parts))
        .route("/parts/{id}", axum::routing::get(get_part::get_part))
        .route("/products", axum::routing::get(list_products::list_products))
        .route("/products/{id}", axum::routing::get(get_product::get_product))
        .route("/products/check-serial", axum::routing::post(check_serial::check_serial))
        .route("/products/register", axum::routing::post(register_product::register_product))
        .merge(tech_routes)
        .merge(admin_routes)
}
