pub mod auth;
pub mod api_docs;
pub mod media;
pub mod notifications;
pub mod work_orders;
pub mod inventory;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/docs", api_docs::router())
        .nest("/work_orders", work_orders_router(state.clone()))
        .nest("/inventory", inventory::router(state.clone()))
        .nest("/notifications", notifications::notifications_router(state.clone()))
        .nest("/media", media::media_router(state))
}

fn work_orders_router(state: AppState) -> Router<AppState> {
    let customer_routes = Router::new()
        .route("/", axum::routing::post(work_orders::create))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    let tech_routes = Router::new()
        .route("/{id}/start", axum::routing::post(work_orders::start))
        .route("/{id}/refuse", axum::routing::post(work_orders::refuse))
        .route("/{id}/complete", axum::routing::post(work_orders::complete))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    let admin_routes = Router::new()
        .route("/{id}/assign", axum::routing::post(work_orders::assign))
        .route("/{id}/cancel", axum::routing::post(work_orders::cancel))
        .route("/{id}/refusal/approve", axum::routing::post(work_orders::approve_refusal))
        .route("/{id}/refusal/deny", axum::routing::post(work_orders::deny_refusal))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    let list_route = Router::new()
        .route("/", axum::routing::get(work_orders::list));

    Router::new()
        .route("/{id}", axum::routing::get(work_orders::get_details))
        .route("/{id}/history", axum::routing::get(work_orders::history))
        .merge(list_route)
        .merge(customer_routes)
        .merge(tech_routes)
        .merge(admin_routes)
}
