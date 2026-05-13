pub mod list_categories;
pub mod get_preferences;
pub mod update_preferences;
pub mod list;
pub mod sync_outbox;

use axum::Router;
use crate::core::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(list::list))
        .route("/preferences", axum::routing::get(get_preferences::get_preferences).put(update_preferences::update_preferences))
        .route("/categories", axum::routing::get(list_categories::list_categories))
        .route("/outbox/sync", axum::routing::post(sync_outbox::sync_outbox))
}
