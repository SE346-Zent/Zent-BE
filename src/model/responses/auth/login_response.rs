use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountStatusEnum {
    Active,
    Pending,
    Locked,
    Inactive,
    Terminated,
    Unknown(i32),
}

impl AccountStatusEnum {
    pub fn from_i32(v: i32) -> Self {
        match v {
            1 => AccountStatusEnum::Active,
            2 => AccountStatusEnum::Pending,
            3 => AccountStatusEnum::Locked,
            4 => AccountStatusEnum::Inactive,
            5 => AccountStatusEnum::Terminated,
            other => AccountStatusEnum::Unknown(other),
        }
    }
}

// TODO: Temp patch; fix later
impl From<AccountStatusEnum> for i32 {
    fn from(value: AccountStatusEnum) -> Self {
        match value {
            AccountStatusEnum::Active => 1,
            AccountStatusEnum::Pending => 2,
            AccountStatusEnum::Locked => 3,
            AccountStatusEnum::Inactive => 4,
            AccountStatusEnum::Terminated => 5,
            AccountStatusEnum::Unknown(other) => other,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponseData {
    pub account_status: AccountStatusEnum,
    pub email: String,
    pub phone: String,
    pub role_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub status_code: u16,
    pub message: String,
    pub data: LoginResponseData,
    pub meta: Option<serde_json::Value>,
}

impl LoginResponse {
    pub fn success(data: LoginResponseData) -> Self {
        Self {
            status_code: 200,
            message: "Login successful".to_string(),
            data,
            meta: None,
        }
    }
}
