use rand::Rng;

/// Generate a secure, random numeric 6-digit One-Time Password (OTP) code.
///
/// # Returns
/// A string representation of the 6-digit code (e.g., "123456").
pub fn generate_6digit_otp() -> String {
    let mut rng = rand::rng();
    let code: u32 = rng.random_range(100_000..=999_999);
    code.to_string()
}
