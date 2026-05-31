use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeConnection {
    pub connection_id: String,
    pub user_id: String,
    pub channels: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceChange {
    pub channel: String,
    pub user_id: String,
    pub status: PresenceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceStatus {
    Connected,
    Disconnected,
}

#[derive(Debug, Default)]
pub struct RealtimeConnections {
    connections: HashMap<String, RealtimeConnection>,
    users: HashMap<String, HashSet<String>>,
    channel_users: HashMap<String, HashMap<String, usize>>,
}

impl RealtimeConnections {
    pub fn register(
        &mut self,
        connection_id: String,
        user_id: String,
        initial_channel: String,
    ) -> Vec<PresenceChange> {
        let mut connection = RealtimeConnection {
            connection_id: connection_id.clone(),
            user_id: user_id.clone(),
            channels: HashSet::new(),
        };
        self.users
            .entry(user_id.clone())
            .or_default()
            .insert(connection_id.clone());
        let change = self.join_channel(&connection_id, &user_id, &initial_channel);
        connection.channels.insert(initial_channel);
        self.connections.insert(connection_id, connection);
        change.into_iter().collect()
    }

    pub fn unregister(&mut self, connection_id: &str) -> Vec<PresenceChange> {
        let Some(connection) = self.connections.remove(connection_id) else {
            return Vec::new();
        };

        if let Some(connections) = self.users.get_mut(&connection.user_id) {
            connections.remove(connection_id);
            if connections.is_empty() {
                self.users.remove(&connection.user_id);
            }
        }

        connection
            .channels
            .into_iter()
            .filter_map(|channel| self.leave_channel(&connection.user_id, &channel))
            .collect()
    }

    pub fn join(&mut self, connection_id: &str, channel: String) -> Option<PresenceChange> {
        let user_id = self.connections.get(connection_id)?.user_id.clone();
        let connection = self.connections.get_mut(connection_id)?;
        if !connection.channels.insert(channel.clone()) {
            return None;
        }
        self.join_channel(connection_id, &user_id, &channel)
    }

    pub fn leave(&mut self, connection_id: &str, channel: &str) -> Option<PresenceChange> {
        let user_id = {
            let connection = self.connections.get_mut(connection_id)?;
            if !connection.channels.remove(channel) {
                return None;
            }
            connection.user_id.clone()
        };
        self.leave_channel(&user_id, channel)
    }

    pub fn channels_for_connection(&self, connection_id: &str) -> HashSet<String> {
        self.connections
            .get(connection_id)
            .map(|connection| connection.channels.clone())
            .unwrap_or_default()
    }

    pub fn channel_user_count(&self, channel: &str, user_id: &str) -> usize {
        self.channel_users
            .get(channel)
            .and_then(|users| users.get(user_id))
            .copied()
            .unwrap_or(0)
    }

    fn join_channel(
        &mut self,
        _connection_id: &str,
        user_id: &str,
        channel: &str,
    ) -> Option<PresenceChange> {
        let count = self
            .channel_users
            .entry(channel.to_owned())
            .or_default()
            .entry(user_id.to_owned())
            .or_insert(0);
        *count += 1;

        (*count == 1).then(|| PresenceChange {
            channel: channel.to_owned(),
            user_id: user_id.to_owned(),
            status: PresenceStatus::Connected,
        })
    }

    fn leave_channel(&mut self, user_id: &str, channel: &str) -> Option<PresenceChange> {
        let users = self.channel_users.get_mut(channel)?;
        let count = users.get_mut(user_id)?;
        *count = count.saturating_sub(1);
        if *count > 0 {
            return None;
        }

        users.remove(user_id);
        if users.is_empty() {
            self.channel_users.remove(channel);
        }

        Some(PresenceChange {
            channel: channel.to_owned(),
            user_id: user_id.to_owned(),
            status: PresenceStatus::Disconnected,
        })
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn user_connection_count(&self, user_id: &str) -> usize {
        self.users.get(user_id).map_or(0, HashSet::len)
    }
}
