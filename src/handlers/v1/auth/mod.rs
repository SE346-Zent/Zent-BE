pub mod forgot_password;
pub mod verify_forgot_password_otp;
pub mod reset_password;
pub mod refresh_token;
pub mod login;
pub mod register;
pub mod verify_otp;
pub mod resend_otp;
pub mod auto_login;
pub mod logout;

pub use auto_login::auto_login_handler;
pub use forgot_password::forgot_password_handler;
pub use verify_forgot_password_otp::verify_forgot_password_otp_handler;
pub use reset_password::reset_password_handler;
pub use refresh_token::refresh_token_handler;
pub use login::login_handler;
pub use register::register_handler;
pub use verify_otp::verify_otp_handler;
pub use resend_otp::resend_otp_handler;
pub use logout::logout_handler;

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
pub use auto_login::__path_auto_login_handler;

use axum::{Router, routing::post};
use crate::core::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/logout", post(logout_handler))
        .route("/register", post(register_handler))
        .route("/verify-otp", post(verify_otp_handler))
        .route("/resend-otp", post(resend_otp_handler))
        .route("/refresh-token", post(refresh_token_handler))
        .route("/forgot-password", post(forgot_password_handler))
        .route("/verify-forgot-password-otp", post(verify_forgot_password_otp_handler))
        .route("/reset-password", post(reset_password_handler))
        .route("/auto-login", post(auto_login_handler))
}
