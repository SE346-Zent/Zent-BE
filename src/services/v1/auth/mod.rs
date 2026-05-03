// Internal logic modules made public for handler orchestration
pub mod login;
pub mod register;
pub mod verify_otp;
pub mod resend_otp;
pub mod refresh_token;
pub mod forgot_password;
pub mod verify_forgot_password_otp;
pub mod reset_password;

#[derive(Clone)]
pub struct AuthService;

impl AuthService {
    pub fn new() -> Self {
        Self
    }
}
