use rand::Rng;

/// Generate a numeric 6-digit OTP code as a string.
pub fn generate_6digit_otp() -> String {
    let mut rng = rand::rng();
    let code: u32 = rng.random_range(100_000..=999_999);
    code.to_string()
}
