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
}

impl PresencePayload {
    pub fn connected(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            status: "connected".to_owned(),
        }
    }

    pub fn disconnected(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            status: "disconnected".to_owned(),
        }
    }

    pub fn into_value(self) -> Value {
        json!(self)
    }
}
