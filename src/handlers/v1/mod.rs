pub mod auth;
pub mod api_docs;
pub mod media;
pub mod notifications;
pub mod work_orders;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/docs", api_docs::router())
        .nest("/work_orders", work_orders_router(state.clone()))
        .nest("/media", media_router(state))
}

fn work_orders_router(state: AppState) -> Router<AppState> {
    // 1. Customer Routes
    let customer_routes = Router::new()
        .route("/", axum::routing::post(work_orders::create))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    // 2. Technician Routes
    let tech_routes = Router::new()
        .route("/{id}/start", axum::routing::post(work_orders::start))
        .route("/{id}/refuse", axum::routing::post(work_orders::refuse))
        .route("/{id}/complete", axum::routing::post(work_orders::complete))
        .route("/{id}/parts", axum::routing::post(work_orders::add_parts))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    // 3. Admin Routes
    let admin_routes = Router::new()
        .route("/{id}/assign", axum::routing::post(work_orders::assign))
        .route("/{id}/cancel", axum::routing::post(work_orders::cancel))
        .route("/{id}/refusal/approve", axum::routing::post(work_orders::approve_refusal))
        .route("/{id}/refusal/deny", axum::routing::post(work_orders::deny_refusal))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin]),
        ));

    // 4. Unified List Route (Shared by all roles - Auth checked inside handler)
    let list_route = Router::new()
        .route("/", axum::routing::get(work_orders::list));

    // 5. Shared/Open Routes
    Router::new()
        .route("/{id}", axum::routing::get(work_orders::get_details))
        .route("/{id}/history", axum::routing::get(work_orders::history))
        .merge(list_route)
        .merge(customer_routes)
        .merge(tech_routes)
        .merge(admin_routes)
}

fn media_router(state: AppState) -> Router<AppState> {
    let closing_form_routes = Router::new()
        .route("/photos", axum::routing::post(media::upload_closing_form_photo))
        .route("/photos/{image_id}", axum::routing::patch(media::update_closing_form_photo))
        .route("/signature", axum::routing::post(media::upload_closing_form_signature))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ));

    Router::new()
        .nest("/work_orders/{id}/closing_form", closing_form_routes)
        .route("/photos/work_orders/{id}", axum::routing::get(media::get_work_order_photo))
        .route("/photos/work_orders", axum::routing::get(media::list_work_order_photos))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Technician]),
        ))
}
