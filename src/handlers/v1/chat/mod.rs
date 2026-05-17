use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::extractor::role_check::require_role;
use crate::entities::roles::Role;

pub mod list_rooms;
pub mod create_room;
pub mod get_messages;
pub mod send_message;
pub mod upload_attachment;
pub mod ws;

pub use list_rooms::__path_list_rooms;
pub use create_room::__path_create_room;
pub use get_messages::__path_get_messages;
pub use send_message::__path_send_message;
pub use upload_attachment::__path_upload_attachment;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/rooms", axum::routing::get(list_rooms::list_rooms).post(create_room::create_room))
        .route("/rooms/{id}/messages", axum::routing::get(get_messages::get_messages).post(send_message::send_message))
        .route("/attachments", axum::routing::post(upload_attachment::upload_attachment))
        .route_layer(middleware::from_fn_with_state(
            state,
            require_role::<AppState>(&[Role::Customer, Role::Technician, Role::Admin, Role::SuperAdmin]),
        ))
}
