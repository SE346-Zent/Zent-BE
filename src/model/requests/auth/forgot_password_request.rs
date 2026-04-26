use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}
