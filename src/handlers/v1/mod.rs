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
}
