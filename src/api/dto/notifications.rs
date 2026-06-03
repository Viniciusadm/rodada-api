use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct NotificationResponse {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub link: Option<String>,
    pub read: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationPreferenceResponse {
    pub key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferenceRequest {
    pub enabled: bool,
}
