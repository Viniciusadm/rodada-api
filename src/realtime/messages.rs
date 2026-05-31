use serde::Deserialize;
use serde_json::{json, Value};

use crate::realtime::RealtimeEvent;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRealtimeMessage {
    Join { channel: String },
    Leave { channel: String },
    Ping,
}

pub fn ack_payload(action: &str, channel: Option<&str>, connection_id: &str) -> Value {
    json!({
        "action": action,
        "channel": channel,
        "connection_id": connection_id,
    })
}

pub fn error_payload(code: &str, message: &str, connection_id: &str) -> Value {
    json!({
        "code": code,
        "message": message,
        "connection_id": connection_id,
    })
}

pub fn pong_payload(connection_id: &str) -> Value {
    json!({
        "connection_id": connection_id,
    })
}

pub fn serialize_event(event: &RealtimeEvent) -> Option<String> {
    serde_json::to_string(event).ok()
}
