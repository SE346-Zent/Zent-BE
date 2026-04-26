use serde::Deserialize;
use validator::Validate;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Validate, IntoParams, ToSchema)]
pub struct ResetPasswordRequest {
    pub reset_token: String,
    #[validate(length(min = 8))]
    pub new_password: String,
}
