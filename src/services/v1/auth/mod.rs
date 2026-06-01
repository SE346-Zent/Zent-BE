//! Business logic for authentication and account management.
//!
//! This module contains the 'decide' functions that encapsulate the core rules
//! for login, registration, password recovery, and session management.

// Internal logic modules made public for handler orchestration
pub mod login;
pub mod register;
pub mod verify_otp;
pub mod resend_otp;
pub mod refresh_token;
pub mod forgot_password;
pub mod verify_forgot_password_otp;
pub mod reset_password;
pub mod logout;
pub mod change_password;
pub mod google_login;
pub mod login_history;

