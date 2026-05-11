pub mod list_categories;
pub mod get_preferences;
pub mod update_preferences;
pub mod list;
pub mod get_detail;
pub mod mark_read;
pub mod mark_all_read;
pub mod sync_outbox;

use axum::Router;
use crate::core::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(list::list))
        .route("/{id}", axum::routing::get(get_detail::get_detail))
        .route("/{id}/read", axum::routing::post(mark_read::mark_read))
        .route("/read-all", axum::routing::post(mark_all_read::mark_all_read))
        .route("/preferences", axum::routing::get(get_preferences::get_preferences).put(update_preferences::update_preferences))
        .route("/categories", axum::routing::get(list_categories::list_categories))
        .route("/outbox/sync", axum::routing::post(sync_outbox::sync_outbox))
}
