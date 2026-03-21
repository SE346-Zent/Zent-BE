use crate::define_api_response;

define_api_response!(RegisterResponse, Option<()>, Option<()>);

impl RegisterResponse {
    pub fn success(message: &str) -> Self {
        Self {
            status_code: 201,
            message: message.to_string(),
            data: None,
            meta: None,
        }
    }
}
