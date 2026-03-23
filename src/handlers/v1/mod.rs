pub mod account;
pub mod auth;
pub mod work_order;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/account", account::router())
        .nest("/auth", auth::router())
        .nest("/work_order", work_order::router())
}
