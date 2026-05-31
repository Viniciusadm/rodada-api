use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::realtime::{RealtimeChannel, RealtimeEventKind};

#[derive(Debug, Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<RealtimeEvent>,
}

impl RealtimeHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self, channel: RealtimeChannel) -> RealtimeSubscription {
        RealtimeSubscription {
            channel_key: channel.key(),
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: RealtimeEvent) {
        let _ = self.sender.send(event);
    }
}

impl Default for RealtimeHub {
    fn default() -> Self {
        Self::new(512)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RealtimeEvent {
    pub channel: String,
    pub kind: RealtimeEventKind,
    pub payload: Value,
}

impl RealtimeEvent {
    pub fn new(channel: RealtimeChannel, kind: RealtimeEventKind, payload: Value) -> Self {
        Self {
            channel: channel.key(),
            kind,
            payload,
        }
    }
}

pub struct RealtimeSubscription {
    channel_key: String,
    receiver: broadcast::Receiver<RealtimeEvent>,
}

impl RealtimeSubscription {
    pub async fn recv(&mut self) -> Result<RealtimeEvent, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            if event.channel == self.channel_key {
                return Ok(event);
            }
        }
    }
}
