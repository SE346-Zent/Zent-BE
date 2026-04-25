use num_enum::{FromPrimitive, IntoPrimitive};
use serde::{Deserialize, Serialize};

/// Account status enum
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    utoipa::ToSchema,
    IntoPrimitive,
    FromPrimitive,
)]
#[serde(from = "i32", into = "i32")]
#[schema(example = 1)]
#[repr(i32)]
pub enum AccountStatusEnum {
    /// Active accounts
    Active = 1,
    /// Accounts pending email verification (Customer only)
    Pending = 2,
    /// Accounts administratively locked down due to policy violations (Customer only)
    Locked = 3,
    /// Accounts administratively deactivated (Technician/Administrator only)
    Inactive = 4,
    /// Accounts deleted (Customer only)
    Terminated = 5,
    /// Unknown account status
    #[num_enum(catch_all)]
    Unknown(i32),
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    /// Account status mapped to database integer ID
    pub account_status: AccountStatusEnum,
    /// Full name
    pub full_name: String,
    /// Email
    pub email: String,
    /// Phone number
    pub phone_number: String,
    /// Role ID
    pub role_id: i32,
}

/// Login response data
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseData {
    /// User info
    pub user: UserInfo,
    /// Access token used to authenticate requests
    pub access_token: String,
    /// Refresh token used to refresh access token
    pub refresh_token: String,
}
