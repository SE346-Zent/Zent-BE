use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::extractor::role_check::require_role;
use crate::entities::roles::Role;

pub mod rooms;
pub mod messages;
pub mod attachments;
pub mod ws;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/rooms", axum::routing::get(rooms::list_rooms).post(rooms::create_room))
        .route("/rooms/{id}/messages", axum::routing::get(messages::get_messages).post(messages::send_message))
        .route("/attachments", axum::routing::post(attachments::upload_attachment))
        .route_layer(middleware::from_fn_with_state(
            state,
            require_role::<AppState>(&[Role::Customer, Role::Technician, Role::Admin, Role::SuperAdmin]),
        ))
}
