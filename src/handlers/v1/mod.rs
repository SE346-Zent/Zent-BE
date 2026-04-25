pub mod auth;
pub mod api_docs;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/docs", api_docs::router())
}
