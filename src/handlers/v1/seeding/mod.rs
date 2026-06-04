pub mod create_warranty;

use axum::{Router, middleware};
use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub fn router(app_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/warranties", axum::routing::post(create_warranty::create_warranty))
        .route_layer(middleware::from_fn_with_state(
            app_state,
            require_role::<AppState>(&[Role::Admin, Role::SuperAdmin]),
        ))
}
