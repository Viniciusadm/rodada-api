use serde::{Deserialize, Serialize};

use crate::application::auth::LoginCommand;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl From<LoginRequest> for LoginCommand {
    fn from(value: LoginRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub user_id: String,
    pub display_name: String,
}
