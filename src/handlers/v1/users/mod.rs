use axum::{Router, routing::{get, patch, post, put}, middleware};

use crate::core::state::AppState;
use crate::entities::roles::Role;
use crate::extractor::role_check::require_role;

pub mod get_me;
pub mod update_me;
pub mod close_account;
pub mod list_users;
pub mod get_user;
pub mod create_user;
pub mod update_user_status;

/// Initialize and return the Axum router for user management.
pub fn router(state: AppState) -> Router<AppState> {
    let generic_routes = Router::new()
        .route("/me", get(get_me::get_me_handler))
        .route("/me", put(update_me::update_me_handler))
        .route("/me/close", post(close_account::close_account_handler));

    let admin_only_routes = Router::new()
        .route("/", get(list_users::list_users_handler))
        .route("/", post(create_user::create_user_handler))
        .route("/{id}", get(get_user::get_user_handler))
        .route("/{id}/status", patch(update_user_status::update_user_status_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_role::<AppState>(&[Role::Admin, Role::SuperAdmin]),
        ));

    Router::new()
        .merge(generic_routes)
        .merge(admin_only_routes)
}
