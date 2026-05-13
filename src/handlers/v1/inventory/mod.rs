pub mod add_parts;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/work_orders/{id}/parts", axum::routing::post(add_parts::add_parts))
        .route_layer(middleware::from_fn_with_state(
            state,
            require_role::<AppState>(&[Role::Technician]),
        ))
}
