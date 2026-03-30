//! Online presence tracking for the friends system and location awareness.

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Events pushed to online users about their friends.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FriendEvent {
    /// A friend came online or went offline.
    FriendPresence {
        username: String,
        friend_code: String,
        online: bool,
        character_name: Option<String>,
        character_class: Option<String>,
        location: Option<String>,
    },
    /// A friend sent you a chat message.
    FriendChat {
        from: String,
        text: String,
        ts: String,
    },
    /// Someone sent you a friend request.
    FriendRequestReceived {
        from_username: String,
        from_tag: String,
    },
    /// Someone accepted your friend request.
    FriendRequestAccepted {
        username: String,
        friend_code: String,
    },
    /// Location chat message (broadcast to players at same location).
    LocationChat {
        from: String,
        character_name: String,
        text: String,
        ts: String,
        location: String,
    },
    /// Location presence update (players arrived/left).
    LocationPresenceUpdate {
        location: String,
        players: Vec<crate::web::protocol::LocationPlayerInfo>,
    },
}

/// Per-user presence info stored in the registry.
#[derive(Debug, Clone)]
pub struct PresenceEntry {
    pub username: String,
    pub friend_code: String,
    pub character_name: Option<String>,
    pub character_class: Option<String>,
    pub character_level: Option<u32>,
    pub location: Option<String>,
    pub adventure_id: Option<String>,
    pub connected_at: DateTime<Utc>,
    /// Channel to push events to this user's WebSocket.
    pub tx: broadcast::Sender<FriendEvent>,
}

/// A single location chat message.
#[derive(Debug, Clone, Serialize)]
pub struct LocationChatMessage {
    pub from: String,
    pub character_name: String,
    pub text: String,
    pub ts: String,
}

const LOCATION_CHAT_BUFFER_SIZE: usize = 50;

/// Shared presence registry with location awareness and chat.
#[derive(Clone)]
pub struct PresenceRegistry {
    inner: Arc<RwLock<HashMap<String, PresenceEntry>>>,
    /// Per-location chat ring buffers (last N messages).
    location_chat: Arc<RwLock<HashMap<String, VecDeque<LocationChatMessage>>>>,
}

impl PresenceRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            location_chat: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a user as online. Returns a receiver for friend events.
    pub async fn connect(
        &self,
        username: String,
        friend_code: String,
    ) -> broadcast::Receiver<FriendEvent> {
        let (tx, rx) = broadcast::channel(64);
        let entry = PresenceEntry {
            username: username.clone(),
            friend_code,
            character_name: None,
            character_class: None,
            character_level: None,
            location: None,
            adventure_id: None,
            connected_at: Utc::now(),
            tx,
        };
        self.inner.write().await.insert(username, entry);
        rx
    }

    /// Remove a user from the registry.
    pub async fn disconnect(&self, username: &str) {
        self.inner.write().await.remove(username);
    }

    /// Update a user's character/location info.
    pub async fn update_presence(
        &self,
        username: &str,
        character_name: Option<String>,
        character_class: Option<String>,
        location: Option<String>,
    ) {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(username) {
            entry.character_name = character_name;
            entry.character_class = character_class;
            entry.location = location;
        }
    }

    /// Update adventure_id and character level.
    pub async fn update_adventure_info(
        &self,
        username: &str,
        adventure_id: Option<String>,
        character_level: Option<u32>,
    ) {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(username) {
            entry.adventure_id = adventure_id;
            entry.character_level = character_level;
        }
    }

    /// Check if a user is online.
    pub async fn is_online(&self, username: &str) -> bool {
        self.inner.read().await.contains_key(username)
    }

    /// Get presence info for a user (if online).
    pub async fn get(&self, username: &str) -> Option<PresenceEntry> {
        self.inner.read().await.get(username).cloned()
    }

    /// Send an event to a specific user (if online).
    pub async fn send_to(&self, username: &str, event: FriendEvent) {
        let map = self.inner.read().await;
        if let Some(entry) = map.get(username) {
            let _ = entry.tx.send(event);
        }
    }

    /// Notify a list of friends about a presence change.
    pub async fn broadcast_to_friends(
        &self,
        friends: &[String],
        event: FriendEvent,
    ) {
        let map = self.inner.read().await;
        for friend in friends {
            if let Some(entry) = map.get(friend.as_str()) {
                let _ = entry.tx.send(event.clone());
            }
        }
    }

    /// Get presence info for multiple users at once.
    pub async fn get_bulk(&self, usernames: &[String]) -> Vec<(String, Option<PresenceEntry>)> {
        let map = self.inner.read().await;
        usernames
            .iter()
            .map(|u| (u.clone(), map.get(u.as_str()).cloned()))
            .collect()
    }

    // --- Location awareness ---

    /// Get all online users at a given location (case-insensitive).
    pub async fn get_users_at_location(&self, location: &str) -> Vec<PresenceEntry> {
        let loc_lower = location.to_lowercase();
        let map = self.inner.read().await;
        map.values()
            .filter(|e| {
                e.location
                    .as_ref()
                    .map(|l| l.to_lowercase() == loc_lower)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Send a FriendEvent to all users at a location (except `exclude`).
    pub async fn broadcast_to_location(
        &self,
        location: &str,
        event: FriendEvent,
        exclude: Option<&str>,
    ) {
        let loc_lower = location.to_lowercase();
        let map = self.inner.read().await;
        for entry in map.values() {
            if let Some(ref loc) = entry.location {
                if loc.to_lowercase() == loc_lower {
                    if exclude.map_or(true, |ex| entry.username != ex) {
                        let _ = entry.tx.send(event.clone());
                    }
                }
            }
        }
    }

    // --- Location chat ---

    /// Append a message to a location's chat buffer and return it.
    pub async fn append_location_chat(
        &self,
        location: &str,
        from: String,
        character_name: String,
        text: String,
    ) -> LocationChatMessage {
        let msg = LocationChatMessage {
            from,
            character_name,
            text,
            ts: Utc::now().to_rfc3339(),
        };
        let loc_key = location.to_lowercase();
        let mut chats = self.location_chat.write().await;
        let buf = chats.entry(loc_key).or_insert_with(VecDeque::new);
        buf.push_back(msg.clone());
        if buf.len() > LOCATION_CHAT_BUFFER_SIZE {
            buf.pop_front();
        }
        msg
    }

    /// Get recent chat messages for a location, filtered to max_age_secs.
    pub async fn get_location_chat_recent(&self, location: &str, max_age_secs: i64) -> Vec<LocationChatMessage> {
        let loc_key = location.to_lowercase();
        let chats = self.location_chat.read().await;
        if let Some(buf) = chats.get(&loc_key) {
            let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_secs);
            buf.iter()
                .filter(|m| {
                    chrono::DateTime::parse_from_rfc3339(&m.ts)
                        .map(|dt| dt.with_timezone(&chrono::Utc) > cutoff)
                        .unwrap_or(true)
                })
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get recent chat messages for a location.
    pub async fn get_location_chat(&self, location: &str) -> Vec<LocationChatMessage> {
        let loc_key = location.to_lowercase();
        let chats = self.location_chat.read().await;
        chats
            .get(&loc_key)
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}
