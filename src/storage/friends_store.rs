//! Friends storage: friend lists, requests, and chat history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::Result;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendsData {
    pub friends: HashSet<String>,
    #[serde(default)]
    pub outgoing_requests: HashSet<String>,
    #[serde(default)]
    pub incoming_requests: HashSet<String>,
}

impl Default for FriendsData {
    fn default() -> Self {
        Self {
            friends: HashSet::new(),
            outgoing_requests: HashSet::new(),
            incoming_requests: HashSet::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInfo {
    pub username: String,
    pub friend_code: String,
    pub online: bool,
    pub character_name: Option<String>,
    pub character_class: Option<String>,
    pub location: Option<String>,
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

pub struct FriendsStore {
    data_dir: PathBuf,
}

impl FriendsStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
        }
    }

    fn friends_path(&self, username: &str) -> PathBuf {
        self.data_dir
            .join("users")
            .join(username)
            .join("friends.json")
    }

    fn chat_dir(&self, username: &str) -> PathBuf {
        self.data_dir.join("users").join(username).join("chats")
    }

    fn chat_path(&self, username: &str, friend: &str) -> PathBuf {
        self.chat_dir(username).join(format!("{}.jsonl", friend))
    }

    pub fn load(&self, username: &str) -> Result<FriendsData> {
        let path = self.friends_path(username);
        if !path.exists() {
            return Ok(FriendsData::default());
        }
        let data = std::fs::read_to_string(&path)?;
        let friends: FriendsData = serde_json::from_str(&data)?;
        Ok(friends)
    }

    pub fn save(&self, username: &str, data: &FriendsData) -> Result<()> {
        let path = self.friends_path(username);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Send a friend request from `from` to `to`.
    pub fn send_request(&self, from: &str, to: &str) -> Result<()> {
        let mut from_data = self.load(from)?;
        let mut to_data = self.load(to)?;

        // Already friends?
        if from_data.friends.contains(to) {
            return Ok(());
        }

        from_data.outgoing_requests.insert(to.to_string());
        to_data.incoming_requests.insert(from.to_string());

        self.save(from, &from_data)?;
        self.save(to, &to_data)?;
        Ok(())
    }

    /// Accept a friend request from `from` (called by `acceptor`).
    pub fn accept_request(&self, acceptor: &str, from: &str) -> Result<bool> {
        let mut acc_data = self.load(acceptor)?;
        let mut from_data = self.load(from)?;

        if !acc_data.incoming_requests.remove(from) {
            return Ok(false); // No such request
        }
        from_data.outgoing_requests.remove(acceptor);

        acc_data.friends.insert(from.to_string());
        from_data.friends.insert(acceptor.to_string());

        self.save(acceptor, &acc_data)?;
        self.save(from, &from_data)?;
        Ok(true)
    }

    /// Decline a friend request from `from` (called by `decliner`).
    pub fn decline_request(&self, decliner: &str, from: &str) -> Result<bool> {
        let mut dec_data = self.load(decliner)?;
        let mut from_data = self.load(from)?;

        if !dec_data.incoming_requests.remove(from) {
            return Ok(false);
        }
        from_data.outgoing_requests.remove(decliner);

        self.save(decliner, &dec_data)?;
        self.save(from, &from_data)?;
        Ok(true)
    }

    /// Remove a friend (bidirectional).
    pub fn remove_friend(&self, user: &str, friend: &str) -> Result<bool> {
        let mut user_data = self.load(user)?;
        let mut friend_data = self.load(friend)?;

        let removed = user_data.friends.remove(friend);
        friend_data.friends.remove(user);

        self.save(user, &user_data)?;
        self.save(friend, &friend_data)?;
        Ok(removed)
    }

    /// Append a chat message (stored on both sides).
    pub fn append_chat(&self, from: &str, to: &str, text: &str) -> Result<ChatMessage> {
        let msg = ChatMessage {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
        };
        let line = serde_json::to_string(&msg)? + "\n";

        // Store on both sides
        for user in &[from, to] {
            let dir = self.chat_dir(user);
            std::fs::create_dir_all(&dir)?;
            let other = if *user == from { to } else { from };
            let path = self.chat_path(user, other);
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            f.write_all(line.as_bytes())?;
        }

        Ok(msg)
    }

    /// Load chat history with a friend, most recent `limit` messages.
    pub fn load_chat(&self, username: &str, friend: &str, limit: usize) -> Result<Vec<ChatMessage>> {
        let path = self.chat_path(username, friend);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read_to_string(&path)?;
        let mut messages: Vec<ChatMessage> = data
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        // Return last `limit` messages
        if messages.len() > limit {
            messages = messages.split_off(messages.len() - limit);
        }
        Ok(messages)
    }
}
