//! HTTP handlers for authentication and account management.
//!
//! This module provides the REST API endpoints for user authentication,
//! registration, and security-related operations.

pub mod forgot_password;
pub mod verify_forgot_password_otp;
pub mod reset_password;
pub mod refresh_token;
pub mod login;
pub mod register;
pub mod verify_otp;
pub mod resend_otp;
pub mod logout;
pub mod change_password;
pub mod google_login;
pub mod login_history;
pub mod set_recovery_email;
pub mod verify_recovery_email;

pub use forgot_password::forgot_password_handler;
pub use verify_forgot_password_otp::verify_forgot_password_otp_handler;
pub use reset_password::reset_password_handler;
pub use refresh_token::refresh_token_handler;
pub use login::login_handler;
pub use register::register_handler;
pub use verify_otp::verify_otp_handler;
pub use resend_otp::resend_otp_handler;
pub use logout::logout_handler;
pub use change_password::change_password_handler;
pub use google_login::google_login_handler;
pub use login_history::login_history_handler;
pub use set_recovery_email::set_recovery_email_handler;
pub use verify_recovery_email::verify_recovery_email_handler;

// Re-export __path_* items for utoipa OpenApi derive
pub use forgot_password::__path_forgot_password_handler;
pub use verify_forgot_password_otp::__path_verify_forgot_password_otp_handler;
pub use reset_password::__path_reset_password_handler;
pub use refresh_token::__path_refresh_token_handler;
pub use login::__path_login_handler;
pub use register::__path_register_handler;
pub use verify_otp::__path_verify_otp_handler;
pub use resend_otp::__path_resend_otp_handler;
pub use logout::__path_logout_handler;
pub use change_password::__path_change_password_handler;
pub use google_login::__path_google_login_handler;
pub use login_history::__path_login_history_handler;
pub use set_recovery_email::__path_set_recovery_email_handler;
pub use verify_recovery_email::__path_verify_recovery_email_handler;

use axum::{Router, routing::post};
use crate::core::state::AppState;

/// Initialize and return the Axum router for the authentication domain.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/login-history", axum::routing::get(login_history_handler))
        .route("/logout", post(logout_handler))
        .route("/register", post(register_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/resend-otp", post(resend_otp_handler))
        .route("/refresh-token", post(refresh_token_handler))
        .route("/forgot-password", post(forgot_password_handler))
        .route("/verify-forgot-password-otp", post(verify_forgot_password_otp_handler))
        .route("/reset-password", post(reset_password_handler))
        .route("/change-password", post(change_password_handler))
        .route("/recovery-email", post(set_recovery_email_handler))
        .route("/verify-recovery-email", post(verify_recovery_email_handler))
        .route("/google-login", post(google_login_handler))
}
