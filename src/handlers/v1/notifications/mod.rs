//! HTTP handlers for the notifications domain.
//!
//! Provides endpoints for retrieving in-app notifications, unread counts,
//! and managing user notification delivery preferences.

pub mod get_preferences;
pub mod update_preferences;
pub mod list;
pub mod unread_count;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::extractor::role_check::require_role;
use crate::entities::roles::Role;

/// Initialize and return the Axum router for the notifications domain.
pub fn notifications_router(app_state: AppState) -> Router<AppState> {
    let preference_routes = Router::new()
        .route("/", axum::routing::get(get_preferences::get_preferences).put(update_preferences::update_preferences))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    Router::new()
        .route("/", axum::routing::get(list::list))
        .route("/unread-count", axum::routing::get(unread_count::get_unread_noti_count))
        .nest("/preferences", preference_routes)
}
