use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

use crate::realtime::{PresenceChange, RealtimeChannel, RealtimeConnections, RealtimeEventKind};

pub const GLOBAL_CHANNEL: &str = "global";

#[derive(Debug, Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<RealtimeEvent>,
    connections: Arc<RwLock<RealtimeConnections>>,
}

impl RealtimeHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            connections: Arc::new(RwLock::new(RealtimeConnections::default())),
        }
    }

    pub fn subscribe(&self, channel: RealtimeChannel) -> RealtimeSubscription {
        self.subscribe_keys([channel.key()])
    }

    pub fn subscribe_connection(
        &self,
        connection_id: impl Into<String>,
        user_id: impl Into<String>,
        channel: RealtimeChannel,
    ) -> (RealtimeSubscription, Vec<PresenceChange>) {
        let connection_id = connection_id.into();
        let user_id = user_id.into();
        let channel_key = channel.key();
        let filters = Arc::new(RwLock::new(HashSet::from([
            channel_key.clone(),
            user_channel(&user_id),
            GLOBAL_CHANNEL.to_owned(),
        ])));
        let changes = self
            .connections
            .write()
            .expect("realtime connections lock poisoned")
            .register(connection_id, user_id, channel_key);

        (
            RealtimeSubscription {
                filters,
                receiver: self.sender.subscribe(),
            },
            changes,
        )
    }

    pub fn subscribe_keys<I>(&self, channels: I) -> RealtimeSubscription
    where
        I: IntoIterator<Item = String>,
    {
        RealtimeSubscription {
            filters: Arc::new(RwLock::new(channels.into_iter().collect())),
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, event: RealtimeEvent) {
        let _ = self.sender.send(event);
    }

    pub fn publish_channel(
        &self,
        channel: RealtimeChannel,
        kind: RealtimeEventKind,
        payload: Value,
    ) {
        self.publish(RealtimeEvent::new(channel, kind, payload));
    }

    pub fn publish_user(&self, user_id: &str, kind: RealtimeEventKind, payload: Value) {
        self.publish(RealtimeEvent::new_key(user_channel(user_id), kind, payload));
    }

    pub fn publish_global(&self, kind: RealtimeEventKind, payload: Value) {
        self.publish(RealtimeEvent::new_key(
            GLOBAL_CHANNEL.to_owned(),
            kind,
            payload,
        ));
    }

    pub fn join_connection(
        &self,
        subscription: &RealtimeSubscription,
        connection_id: &str,
        channel: RealtimeChannel,
    ) -> Option<PresenceChange> {
        let channel_key = channel.key();
        subscription.add_channel(channel_key.clone());
        self.connections
            .write()
            .expect("realtime connections lock poisoned")
            .join(connection_id, channel_key)
    }

    pub fn leave_connection(
        &self,
        subscription: &RealtimeSubscription,
        connection_id: &str,
        channel: &RealtimeChannel,
    ) -> Option<PresenceChange> {
        let channel_key = channel.key();
        subscription.remove_channel(&channel_key);
        self.connections
            .write()
            .expect("realtime connections lock poisoned")
            .leave(connection_id, &channel_key)
    }

    pub fn unregister_connection(&self, connection_id: &str) -> Vec<PresenceChange> {
        self.connections
            .write()
            .expect("realtime connections lock poisoned")
            .unregister(connection_id)
    }

    pub fn connection_count(&self) -> usize {
        self.connections
            .read()
            .expect("realtime connections lock poisoned")
            .connection_count()
    }

    pub fn user_connection_count(&self, user_id: &str) -> usize {
        self.connections
            .read()
            .expect("realtime connections lock poisoned")
            .user_connection_count(user_id)
    }

    pub fn channel_user_count(&self, channel: &str, user_id: &str) -> usize {
        self.connections
            .read()
            .expect("realtime connections lock poisoned")
            .channel_user_count(channel, user_id)
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
        Self::new_key(channel.key(), kind, payload)
    }

    pub fn new_key(channel: String, kind: RealtimeEventKind, payload: Value) -> Self {
        Self {
            channel,
            kind,
            payload,
        }
    }
}

pub struct RealtimeSubscription {
    filters: Arc<RwLock<HashSet<String>>>,
    receiver: broadcast::Receiver<RealtimeEvent>,
}

impl RealtimeSubscription {
    pub async fn recv(&mut self) -> Result<RealtimeEvent, broadcast::error::RecvError> {
        loop {
            let event = self.receiver.recv().await?;
            if self.accepts(&event.channel) {
                return Ok(event);
            }
        }
    }

    pub fn add_channel(&self, channel: String) {
        self.filters
            .write()
            .expect("realtime subscription lock poisoned")
            .insert(channel);
    }

    pub fn remove_channel(&self, channel: &str) {
        self.filters
            .write()
            .expect("realtime subscription lock poisoned")
            .remove(channel);
    }

    fn accepts(&self, channel: &str) -> bool {
        self.filters
            .read()
            .expect("realtime subscription lock poisoned")
            .contains(channel)
    }
}

pub fn user_channel(user_id: &str) -> String {
    format!("user:{user_id}")
}
