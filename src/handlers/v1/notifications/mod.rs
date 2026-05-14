pub mod get_preferences;
pub mod update_preferences;
pub mod list;
pub mod unread_count;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::extractor::role_check::require_role;
use crate::entities::roles::Role;

pub fn router(state: AppState) -> Router<AppState> {
    let pref_routes = Router::new()
        .route("/", axum::routing::get(get_preferences::get_preferences).put(update_preferences::update_preferences))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Customer]),
        ));

    Router::new()
        .route("/", axum::routing::get(list::list))
        .route("/unread-count", axum::routing::get(unread_count::get_unread_noti_count))
        .nest("/preferences", pref_routes)
}
