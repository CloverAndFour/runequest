//! Server-side session management — tracks active adventure per account.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// How long before an inactive session expires (60 minutes).
const SESSION_TIMEOUT_SECS: i64 = 3600;

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub username: String,
    pub adventure_id: String,
    pub activated_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl ActiveSession {
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_activity).num_seconds();
        elapsed > SESSION_TIMEOUT_SECS
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Global session registry — one active adventure per username.
#[derive(Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, ActiveSession>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Activate an adventure for a user. Deactivates any previous adventure.
    /// Returns the previously active adventure_id if one was deactivated.
    pub async fn activate(
        &self,
        username: &str,
        adventure_id: &str,
    ) -> Option<String> {
        let mut sessions = self.sessions.write().await;
        let previous = sessions.remove(username).map(|s| s.adventure_id);
        sessions.insert(username.to_string(), ActiveSession {
            username: username.to_string(),
            adventure_id: adventure_id.to_string(),
            activated_at: Utc::now(),
            last_activity: Utc::now(),
        });
        previous
    }

    /// Deactivate the current session for a user.
    /// Returns the deactivated adventure_id if one was active.
    pub async fn deactivate(&self, username: &str) -> Option<String> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(username).map(|s| s.adventure_id)
    }

    /// Get the active session for a user, checking expiry.
    /// Returns None if no session or if expired (auto-cleans expired sessions).
    pub async fn get_active(&self, username: &str) -> Option<ActiveSession> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get(username) {
            if session.is_expired() {
                sessions.remove(username);
                return None;
            }
            return Some(session.clone());
        }
        None
    }

    /// Touch the session (update last_activity). Returns the adventure_id if active.
    /// Returns None if no active session or expired.
    pub async fn touch(&self, username: &str) -> Option<String> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(username) {
            if session.is_expired() {
                sessions.remove(username);
                return None;
            }
            session.touch();
            return Some(session.adventure_id.clone());
        }
        None
    }

    /// Get session info without touching (for status queries).
    pub async fn peek(&self, username: &str) -> Option<ActiveSession> {
        let sessions = self.sessions.read().await;
        sessions.get(username).filter(|s| !s.is_expired()).cloned()
    }
}
