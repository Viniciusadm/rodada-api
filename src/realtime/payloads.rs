use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize)]
pub struct MatchEventPayload {
    pub match_id: String,
}

impl MatchEventPayload {
    pub fn new(match_id: impl Into<String>) -> Self {
        Self {
            match_id: match_id.into(),
        }
    }

    pub fn into_value(self) -> Value {
        json!(self)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PresencePayload {
    pub user_id: String,
    pub status: String,
    pub connection_id: Option<String>,
}

impl PresencePayload {
    pub fn connected(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            status: "connected".to_owned(),
            connection_id: None,
        }
    }

    pub fn disconnected(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            status: "disconnected".to_owned(),
            connection_id: None,
        }
    }

    pub fn with_connection_id(mut self, connection_id: impl Into<String>) -> Self {
        self.connection_id = Some(connection_id.into());
        self
    }

    pub fn into_value(self) -> Value {
        json!(self)
    }
}
