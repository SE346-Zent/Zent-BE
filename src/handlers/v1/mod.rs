pub mod auth;
pub mod api_docs;
pub mod media;
pub mod work_orders;

use axum::Router;
use crate::core::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/docs", api_docs::router())
        .nest("/work_orders", work_orders_router())
        .nest("/media", media_router())
}

fn work_orders_router() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::post(work_orders::create).get(work_orders::list))
        .route("/{id}", axum::routing::get(work_orders::get_details))
        .route("/{id}/assign", axum::routing::post(work_orders::assign))
        .route("/{id}/schedule", axum::routing::post(work_orders::schedule))
        .route("/{id}/start", axum::routing::post(work_orders::start))
        .route("/{id}/refuse", axum::routing::post(work_orders::refuse))
        .route("/{id}/cancel", axum::routing::post(work_orders::cancel))
        .route("/{id}/complete", axum::routing::post(work_orders::complete))
        .route("/{id}/history", axum::routing::get(work_orders::history))
        .route("/{id}/parts", axum::routing::post(work_orders::add_parts))
        .route("/{id}/refusal/approve", axum::routing::post(work_orders::approve_refusal))
        .route("/{id}/refusal/deny", axum::routing::post(work_orders::deny_refusal))
}

fn media_router() -> Router<AppState> {
    Router::new()
        .route("/photos/work_orders/{id}/upload", axum::routing::post(media::upload_work_order_photo))
        .route("/photos/work_orders/{id}", axum::routing::get(media::get_work_order_photo))
        .route("/photos/work_orders", axum::routing::get(media::list_work_order_photos))
        .route("/signatures/work_orders/{id}/upload", axum::routing::post(media::upload_work_order_signature))
}
